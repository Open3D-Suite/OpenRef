# SPDX-License-Identifier: GPL-3.0-or-later
"""The common OPEN3D tool of a repository: its checks and its version.

    python -I tools/repo.py check               the quick checks (what tools/check.ps1 starts)
    python -I tools/repo.py check --full        also the slow ones (tools/check.ps1 -Full)
    python -I tools/repo.py version             print the version
    python -I tools/repo.py version set X.Y.Z   write a new version into every place that holds it

What is specific to the repository is in tools/repo.toml. This file is the same in every OPEN3D
repository. Only the standard library is used; it needs Python 3.11 or newer.
"""

import importlib.util
import os
import re
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path

REPO = Path(__file__).absolute().parent.parent
SETTINGS = "tools/repo.toml"
# The tables of the settings file and the keys of each.
TABLES = {
    "version": {"source", "places", "after_set"},
    "check": {"name", "run", "full"},
    "licence_header": {"exclude"},
    "private_data": {"allow"},
}

LICENCE_LINE = "SPDX-License-Identifier: GPL-3.0-or-later"
SOURCES = {".rs", ".py", ".ps1", ".ms", ".mcr", ".c", ".h", ".cpp", ".hpp", ".js", ".css", ".html"}
CLIPPY_LINTS = {
    "cognitive_complexity": "deny",
    "too_many_arguments": "deny",
    "wildcard_imports": "deny",
    "dbg_macro": "warn",
    "todo": "warn",
}

# A list of words that only its owner's computer has: one per line, never committed. It is looked for
# in the repository's folder and in every folder above it.
PRIVATE_LIST = "open3d-private-terms.txt"
# A folder of a Windows user, on a drive or on a network path (through localhost or any other name);
# its group is the name of the user.
USER_FOLDER = re.compile(r"(?i)(?:\b[a-z]:|\\{2,}[\w.-]+(?:\\+[\w$-]+)?)[\\/]+users[\\/]+(\w[\w.-]*)")
PLACEHOLDER_USERS = {"public", "default", "user", "username", "name", "you"}


class Failure(Exception):
    """Something the person has to put right: the settings, a missing program, a wrong argument."""


def say(text=""):
    print(text, flush=True)


def counted(number, word):
    return f"{number} {word}" + ("" if number == 1 else "s")


# ---------------------------------------------------------------------------------------- settings


def load_settings(repo):
    path = repo / SETTINGS
    if not path.is_file():
        raise Failure(f"{SETTINGS} was not found")
    try:
        settings = tomllib.loads(path.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as error:
        raise Failure(f"{SETTINGS}: {error}") from error
    for table in settings:
        if table not in TABLES:
            raise Failure(f"{SETTINGS}: unknown table [{table}]")
        if isinstance(settings[table], list) != (table == "check"):
            raise Failure(f"{SETTINGS}: each check is written under [[check]], the rest under [{table}]")
        for entry in settings[table] if table == "check" else [settings[table]]:
            unknown = sorted(set(entry) - TABLES[table])
            if unknown:
                raise Failure(f"{SETTINGS}: unknown key in [{table}]: {', '.join(unknown)}")
    version = settings.get("version", {})
    if "source" not in version:
        raise Failure(f"{SETTINGS}: [version] needs a source")
    for place in [version["source"], *version.get("places", [])]:
        if set(place) != {"file", "pattern"}:
            raise Failure(f"{SETTINGS}: a version place is a file and a pattern")
        if pattern(place["pattern"]).groups != 1:
            raise Failure(f"{SETTINGS}: the pattern {place['pattern']} needs exactly one group")
    for step in settings.get("check", []):
        if not step.get("name") or not step.get("run"):
            raise Failure(f"{SETTINGS}: a check needs a name and a run list")
    for allowed in settings.get("private_data", {}).get("allow", []):
        if not allowed.get("pattern") or not allowed.get("reason"):
            raise Failure(f"{SETTINGS}: what [private_data] allows is a pattern and the reason for it")
        pattern(allowed["pattern"])
    return settings


def pattern(text):
    try:
        return re.compile(text, re.MULTILINE)
    except re.error as error:
        raise Failure(f"{SETTINGS}: the pattern {text} is not valid: {error}") from error


def read_text(path):
    """The text of a file, with its line endings as they are."""
    return path.read_bytes().decode("utf-8")


def tracked_files(repo):
    """The files Git tracks in the repository, as paths with forward slashes."""
    git = shutil.which("git")
    if not git:
        raise Failure("git was not found")
    done = subprocess.run([git, "ls-files", "-z"], cwd=repo, capture_output=True)  # noqa: S603 - git with fixed arguments
    if done.returncode != 0:
        raise Failure("git ls-files failed: " + done.stderr.decode("utf-8", errors="replace").strip())
    return [name for name in done.stdout.decode("utf-8").split("\0") if name and (repo / name).is_file()]


# ------------------------------------------------------------------------------------- the version


def read_version(repo, settings):
    source = settings["version"]["source"]
    found = pattern(source["pattern"]).search(read_text(repo / source["file"]))
    if not found:
        raise Failure(f"{source['file']}: no version found with the pattern {source['pattern']}")
    return found.group(1)


def version_step(repo, settings, _files):
    """The version in its source and in every other place that repeats it is the same."""
    version = read_version(repo, settings)
    source = settings["version"]["source"]["file"]
    places = settings["version"].get("places", [])
    problems = []
    for place in places:
        values = [found.group(1) for found in pattern(place["pattern"]).finditer(read_text(repo / place["file"]))]
        if not values:
            problems.append(f"{place['file']}: nothing matches the pattern {place['pattern']}")
        problems += [f"{place['file']} says {value}; {source} says {version}" for value in values if value != version]
    return problems, "" if problems else f"{version} in {counted(len(places) + 1, 'place')}"


def set_version(repo, settings, version):
    """Writes the version into its source and into every other place, then runs what follows."""
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise Failure(f"'{version}' is not a version: it is three numbers, like 1.2.3")

    def put(found):
        whole, start = found.group(0), found.start(0)
        return whole[: found.start(1) - start] + version + whole[found.end(1) - start :]

    source = settings["version"]["source"]
    texts = {}
    # In the source only the first match is the version; in the other places every match is.
    for place, count in [(source, 1)] + [(place, 0) for place in settings["version"].get("places", [])]:
        path = repo / place["file"]
        text = texts[path] if path in texts else read_text(path)
        texts[path], changed = pattern(place["pattern"]).subn(put, text, count=count)
        if not changed:
            raise Failure(f"{place['file']}: nothing matches the pattern {place['pattern']}; nothing was written")
    for path, text in texts.items():
        path.write_bytes(text.encode("utf-8"))
        say(f"  wrote {path.relative_to(repo).as_posix()}")
    for args in settings["version"].get("after_set", []):
        say("  > " + " ".join(args))
        if subprocess.run(command(args), cwd=repo).returncode != 0:  # noqa: S603 - a command of tools/repo.toml
            raise Failure(f"{args[0]} failed")
    problems, places = version_step(repo, settings, None)
    if problems:
        raise Failure("the places do not agree after writing:\n  " + "\n  ".join(problems))
    say(f"The version is {places}.")
    return 0


# ------------------------------------------------------------------------------- the built-in steps


def licence_step(repo, settings, files):
    """Every source file carries the licence line in its first lines."""
    excluded = tuple(folder.rstrip("/") + "/" for folder in settings.get("licence_header", {}).get("exclude", []))
    sources = [name for name in files if Path(name).suffix.lower() in SOURCES and not name.startswith(excluded)]
    problems = []
    for name in sources:
        with open(repo / name, encoding="utf-8", errors="replace") as file:
            head = [file.readline() for _ in range(5)]
        if not any(LICENCE_LINE in line for line in head):
            problems.append(f"{name}: no '{LICENCE_LINE}' line in its first five lines")
    return problems, counted(len(sources), "file")


def lints_step(repo, _settings, files):
    """Every Cargo.toml carries the common lints, itself or through its workspace."""
    manifests = {name: tomllib.loads(read_text(repo / name)) for name in files if Path(name).name == "Cargo.toml"}
    problems = []
    shared = False
    for name, manifest in manifests.items():
        if "lints" in manifest.get("workspace", {}):
            shared = True
            problems += missing_lints(name, manifest["workspace"]["lints"])
    for name, manifest in manifests.items():
        if "package" not in manifest:
            continue
        if manifest.get("lints", {}).get("workspace") is not True:
            problems += missing_lints(name, manifest.get("lints", {}))
        elif not shared:
            problems.append(f"{name}: takes its lints from a workspace that has none")
    return problems, f"{len(manifests)} Cargo.toml"


def missing_lints(name, lints):
    def level(value):
        return value.get("level") if isinstance(value, dict) else value

    problems = []
    if level(lints.get("rust", {}).get("unsafe_code")) not in ("forbid", "deny"):
        problems.append(f'{name}: unsafe_code must be "forbid" or "deny"')
    for lint, wanted in CLIPPY_LINTS.items():
        if level(lints.get("clippy", {}).get(lint)) != wanted:
            problems.append(f'{name}: clippy\'s {lint} must be "{wanted}"')
    return problems


def private_terms(repo):
    """The private list of this computer, if there is one: (its line number, the term in lower case)."""
    for folder in [repo, *repo.parents]:
        path = folder / PRIVATE_LIST
        if path.is_file():
            lines = enumerate(path.read_text(encoding="utf-8-sig").splitlines(), 1)
            return [(n, line.strip().lower()) for n, line in lines if line.strip() and not line.startswith("#")]
    return None


def private_step(repo, settings, files):
    """No text file holds the folder of a Windows user or a term of the private list."""
    terms = private_terms(repo)
    allowed = [pattern(entry["pattern"]) for entry in settings.get("private_data", {}).get("allow", [])]
    problems = []
    for name in files:
        data = (repo / name).read_bytes()
        if data[:2] in (b"\xff\xfe", b"\xfe\xff"):
            text = data.decode("utf-16", errors="replace")
        elif b"\0" in data:
            continue  # not a text file
        else:
            text = data.decode("utf-8", errors="replace")
        for number, line in enumerate(text.splitlines(), 1):
            excused = [found.span() for allow in allowed for found in allow.finditer(line)]
            for start, end, what in private_in_line(line, terms or []):
                if not any(a < end and start < b for a, b in excused):
                    problems.append(f"{name}:{number}: {what}")
    listed = "no private list on this computer" if terms is None else counted(len(terms), "private term")
    return problems, f"{counted(len(files), 'file')}, {listed}"


def private_in_line(line, terms):
    """What a line holds that belongs to one computer: (start, end, what it is)."""
    found = []
    for user in USER_FOLDER.finditer(line):
        if user.group(1).lower() not in PLACEHOLDER_USERS:
            found.append((user.start(), user.end(), "the folder of a Windows user"))
    lowered = line.lower()
    for number, term in terms:
        start = lowered.find(term)
        while start >= 0:
            found.append((start, start + len(term), f"term {number} of {PRIVATE_LIST}"))
            start = lowered.find(term, start + 1)
    return found


# -------------------------------------------------------------------------------------- the checks


def command(args):
    """The command to start: its program, given by name, is looked for on this computer."""
    name, rest = args[0], list(args[1:])
    if name == "python":
        return [sys.executable, "-I", *rest]
    cargo_bin = Path(os.environ.get("CARGO_HOME") or Path.home() / ".cargo") / "bin"
    found = shutil.which(name) or shutil.which(name, path=str(cargo_bin))
    if found:
        return [found, *rest]
    if name == "ruff" and importlib.util.find_spec("ruff"):
        return [sys.executable, "-I", "-m", "ruff", *rest]
    raise Failure(f"{name} was not found: it is not on PATH" + (" and not in this Python" if name == "ruff" else ""))


def run_step(repo, args):
    """Runs a command of the settings file in the repository's folder; its output is shown if it fails."""
    try:
        real = command(args)
    except Failure as failure:
        return [str(failure)], ""
    done = subprocess.run(real, cwd=repo, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)  # noqa: S603 - a command of tools/repo.toml
    if done.returncode == 0:
        return [], ""
    return done.stdout.decode("utf-8", errors="replace").splitlines() or [f"exit code {done.returncode}"], ""


def check(repo, settings, full):
    """Runs every step, the built-in ones first, and says how each went. Returns 0 when all passed."""
    files = tracked_files(repo)
    built_in = [("version", version_step), ("licence header", licence_step), ("private data", private_step)]
    if "Cargo.toml" in files:
        built_in.insert(2, ("common lints", lints_step))
    steps = [(name, False, lambda step=step: step(repo, settings, files)) for name, step in built_in]
    for listed in settings.get("check", []):
        steps.append((listed["name"], listed.get("full", False), lambda args=listed["run"]: run_step(repo, args)))

    say(f"Checking {repo.name}" + (" (with -Full)" if full else ""))
    failed, skipped = [], 0
    for name, slow, step in steps:
        if slow and not full:
            say(f"  skip  {name} (only with -Full)")
            skipped += 1
            continue
        if sys.stdout.isatty():
            print(f"  ....  {name}", end="\r", flush=True)
        started = time.monotonic()
        try:
            problems, note = step()
        except (Failure, OSError, ValueError) as failure:
            problems, note = [str(failure)], ""
        seconds = time.monotonic() - started
        details = ", ".join(part for part in (note, f"{seconds:.0f} s" if seconds >= 1 else "") if part)
        say(f"  {'FAIL' if problems else 'ok  '}  {name}" + (f" ({details})" if details else ""))
        for problem in problems:
            say(f"        {problem}")
        if problems:
            failed.append(name)

    ran = len(steps) - skipped
    left = f" {skipped} more only with -Full." if skipped else ""
    if failed:
        say(f"\n{len(failed)} of {ran} checks FAILED: {', '.join(failed)}.{left}")
        return 1
    say(f"\nAll {ran} checks passed.{left}")
    return 0


def main(arguments):
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(errors="replace")
    try:
        if arguments in (["check"], ["check", "--full"]):
            return check(REPO, load_settings(REPO), full=len(arguments) == 2)
        if arguments == ["version"]:
            say(read_version(REPO, load_settings(REPO)))
            return 0
        if len(arguments) == 3 and arguments[:2] == ["version", "set"]:
            return set_version(REPO, load_settings(REPO), arguments[2])
    except (Failure, OSError) as failure:
        say(f"repo.py: {failure}")
        return 2
    say(__doc__.strip())
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
