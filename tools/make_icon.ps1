# SPDX-License-Identifier: GPL-3.0-or-later
# Draws OpenRef's icon: a red push pin on the dark rounded square every Open3D app icon uses.
#   powershell -NoProfile -File tools\make_icon.ps1
# Writes assets\icon.png (1024 px, the master), assets\icon-256.png (the window icon) and
# assets\icon.ico (16 to 256 px, for the executable).

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$assets = Join-Path $root 'assets'
New-Item -ItemType Directory -Force $assets | Out-Null

function Rounded([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
    $p = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = 2 * $r
    $p.AddArc($x, $y, $d, $d, 180, 90)
    $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $p.CloseFigure()
    $p
}

function Color([int]$r, [int]$g, [int]$b, [int]$a = 255) { [System.Drawing.Color]::FromArgb($a, $r, $g, $b) }

# Light from the left: each part of the pin is shaded across its width.
function Shade([float]$x, [float]$w, $light, $mid, $dark) {
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        (New-Object System.Drawing.PointF($x, 0)), (New-Object System.Drawing.PointF(($x + $w), 0)), $light, $dark)
    $blend = New-Object System.Drawing.Drawing2D.ColorBlend(3)
    $blend.Colors = @($light, $mid, $dark)
    $blend.Positions = @([float]0, [float]0.45, [float]1)
    $brush.InterpolationColors = $blend
    $brush
}

$size = 1024
$bmp = New-Object System.Drawing.Bitmap $size, $size
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = 'AntiAlias'
$g.Clear([System.Drawing.Color]::Transparent)

# The suite's dark rounded square.
$g.FillPath((New-Object System.Drawing.SolidBrush (Color 34 35 42)), (Rounded 24 24 976 976 220))

# The pin is drawn upright around (0, 0), then turned so its point goes to the lower left.
$g.TranslateTransform(540, 500)
$g.RotateTransform(38)

$light = Color 255 112 112; $mid = Color 230 57 70; $dark = Color 160 28 40

# Needle: a thin steel spike.
$needle = New-Object System.Drawing.Drawing2D.GraphicsPath
$needle.AddPolygon(@(
    (New-Object System.Drawing.PointF(-15, 60)), (New-Object System.Drawing.PointF(15, 60)),
    (New-Object System.Drawing.PointF(3, 395)), (New-Object System.Drawing.PointF(-3, 395))))
$g.FillPath((Shade -15 30 (Color 240 242 246) (Color 190 194 202) (Color 120 124 134)), $needle)

# Flange: the wide disc at the bottom of the body.
$g.FillPath((Shade -205 410 $light $mid $dark), (Rounded -205 0 410 80 40))
# Body: narrows from the flange up to the head.
$body = New-Object System.Drawing.Drawing2D.GraphicsPath
$body.AddPolygon(@(
    (New-Object System.Drawing.PointF(-80, 15)), (New-Object System.Drawing.PointF(80, 15)),
    (New-Object System.Drawing.PointF(115, -235)), (New-Object System.Drawing.PointF(-115, -235))))
$g.FillPath((Shade -115 230 $light $mid $dark), $body)
# Head: the wide cap on top.
$g.FillPath((Shade -175 350 $light $mid $dark), (Rounded -175 -330 350 110 55))
# A highlight on the head.
$g.FillPath((New-Object System.Drawing.SolidBrush (Color 255 255 255 70)), (Rounded -135 -312 120 26 13))

$g.Dispose()
$master = Join-Path $assets 'icon.png'
$bmp.Save($master, [System.Drawing.Imaging.ImageFormat]::Png)

function Resized($source, [int]$px) {
    $small = New-Object System.Drawing.Bitmap $px, $px
    $sg = [System.Drawing.Graphics]::FromImage($small)
    $sg.InterpolationMode = 'HighQualityBicubic'
    $sg.PixelOffsetMode = 'HighQuality'
    $sg.DrawImage($source, 0, 0, $px, $px)
    $sg.Dispose()
    $small
}

function PngBytes($image) {
    $ms = New-Object System.IO.MemoryStream
    $image.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    # The comma keeps PowerShell from unrolling the bytes into the pipeline one by one.
    return , $ms.ToArray()
}

$window = Resized $bmp 256
$window.Save((Join-Path $assets 'icon-256.png'), [System.Drawing.Imaging.ImageFormat]::Png)

# The .ico: one PNG per size (as OpenView's, which the resource compiler takes).
$sizes = @(256, 128, 64, 48, 32, 24, 16)
$images = New-Object 'System.Collections.Generic.List[byte[]]'
foreach ($px in $sizes) { $images.Add([byte[]](PngBytes (Resized $bmp $px))) }
$ico = New-Object System.IO.MemoryStream
$w = New-Object System.IO.BinaryWriter $ico
$w.Write([UInt16]0); $w.Write([UInt16]1); $w.Write([UInt16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $px = $sizes[$i]; $data = $images[$i]
    $side = if ($px -ge 256) { 0 } else { $px }
    $w.Write([byte]$side); $w.Write([byte]$side); $w.Write([byte]0); $w.Write([byte]0)
    $w.Write([UInt16]1); $w.Write([UInt16]32); $w.Write([UInt32]$data.Length); $w.Write([UInt32]$offset)
    $offset += $data.Length
}
foreach ($data in $images) { $w.Write($data) }
$w.Flush()
[System.IO.File]::WriteAllBytes((Join-Path $assets 'icon.ico'), $ico.ToArray())
$bmp.Dispose(); $window.Dispose()
Write-Output "wrote $assets\icon.png, icon-256.png and icon.ico"
