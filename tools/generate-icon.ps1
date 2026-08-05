param(
    [string]$OutputPath = "$PSScriptRoot\..\assets\hzpilot.ico",
    [string]$PngPath = "$PSScriptRoot\..\assets\hzpilot.png",
    [string]$SocialPreviewPath = "$PSScriptRoot\..\assets\social-preview.png"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

function New-IconFrame {
    param([int]$Size)

    $bitmap = [System.Drawing.Bitmap]::new(
        $Size,
        $Size,
        [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
    )
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $brush = $null
    $pen = $null
    $stream = $null

    try {
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $graphics.Clear([System.Drawing.Color]::Transparent)

        $margin = [single]($Size * 0.06)
        $diameter = [single]($Size - (2 * $margin))
        $brush = [System.Drawing.SolidBrush]::new(
            [System.Drawing.Color]::FromArgb(255, 14, 116, 144)
        )
        $graphics.FillEllipse($brush, $margin, $margin, $diameter, $diameter)

        $penWidth = [single][Math]::Max(1.4, $Size * 0.075)
        $pen = [System.Drawing.Pen]::new([System.Drawing.Color]::White, $penWidth)
        $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
        $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
        $pen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round

        $points = [System.Drawing.PointF[]]@(
            [System.Drawing.PointF]::new([single]($Size * 0.18), [single]($Size * 0.53)),
            [System.Drawing.PointF]::new([single]($Size * 0.34), [single]($Size * 0.53)),
            [System.Drawing.PointF]::new([single]($Size * 0.42), [single]($Size * 0.30)),
            [System.Drawing.PointF]::new([single]($Size * 0.55), [single]($Size * 0.70)),
            [System.Drawing.PointF]::new([single]($Size * 0.64), [single]($Size * 0.46)),
            [System.Drawing.PointF]::new([single]($Size * 0.82), [single]($Size * 0.46))
        )
        $graphics.DrawLines($pen, $points)

        $stream = [System.IO.MemoryStream]::new()
        $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
        [pscustomobject]@{
            Size = $Size
            Bytes = $stream.ToArray()
        }
    }
    finally {
        if ($null -ne $stream) { $stream.Dispose() }
        if ($null -ne $pen) { $pen.Dispose() }
        if ($null -ne $brush) { $brush.Dispose() }
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

$frames = @(16, 20, 24, 32, 48, 64, 128, 256) | ForEach-Object {
    New-IconFrame -Size $_
}

$largeFrame = $frames | Where-Object { $_.Size -eq 256 } | Select-Object -First 1
[System.IO.File]::WriteAllBytes($PngPath, [byte[]]$largeFrame.Bytes)

$outputDirectory = Split-Path -Parent $OutputPath
[System.IO.Directory]::CreateDirectory($outputDirectory) | Out-Null
$file = [System.IO.File]::Create($OutputPath)
$writer = [System.IO.BinaryWriter]::new($file)

try {
    $writer.Write([uint16]0)
    $writer.Write([uint16]1)
    $writer.Write([uint16]$frames.Count)

    $offset = 6 + (16 * $frames.Count)
    foreach ($frame in $frames) {
        $dimension = if ($frame.Size -eq 256) { 0 } else { $frame.Size }
        $writer.Write([byte]$dimension)
        $writer.Write([byte]$dimension)
        $writer.Write([byte]0)
        $writer.Write([byte]0)
        $writer.Write([uint16]1)
        $writer.Write([uint16]32)
        $writer.Write([uint32]$frame.Bytes.Length)
        $writer.Write([uint32]$offset)
        $offset += $frame.Bytes.Length
    }

    foreach ($frame in $frames) {
        $writer.Write([byte[]]$frame.Bytes)
    }
}
finally {
    $writer.Dispose()
    $file.Dispose()
}

$preview = [System.Drawing.Bitmap]::new(
    1280,
    640,
    [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
)
$previewGraphics = [System.Drawing.Graphics]::FromImage($preview)
$iconStream = [System.IO.MemoryStream]::new([byte[]]$largeFrame.Bytes)
$iconImage = [System.Drawing.Image]::FromStream($iconStream)
$titleFont = [System.Drawing.Font]::new("Segoe UI Semibold", 86)
$subtitleFont = [System.Drawing.Font]::new("Segoe UI", 30)
$metaFont = [System.Drawing.Font]::new("Segoe UI", 20)
$backgroundBrush = [System.Drawing.SolidBrush]::new(
    [System.Drawing.Color]::FromArgb(255, 6, 28, 37)
)
$titleBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
$subtitleBrush = [System.Drawing.SolidBrush]::new(
    [System.Drawing.Color]::FromArgb(255, 190, 221, 227)
)
$accentPen = [System.Drawing.Pen]::new(
    [System.Drawing.Color]::FromArgb(255, 34, 211, 238),
    5
)

try {
    $previewGraphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $previewGraphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $previewGraphics.FillRectangle($backgroundBrush, 0, 0, 1280, 640)
    $previewGraphics.DrawImage($iconImage, 100, 160, 320, 320)
    $previewGraphics.DrawString("HzPilot", $titleFont, $titleBrush, 500, 150)
    $previewGraphics.DrawLine($accentPen, 505, 275, 1110, 275)
    $previewGraphics.DrawString(
        "Switch monitor refresh rates",
        $subtitleFont,
        $subtitleBrush,
        505,
        310
    )
    $previewGraphics.DrawString(
        "from the Windows tray.",
        $subtitleFont,
        $subtitleBrush,
        505,
        355
    )
    $previewGraphics.DrawString(
        "Native Rust + Win32  |  Windows 10/11  |  MIT",
        $metaFont,
        $subtitleBrush,
        505,
        435
    )
    $preview.Save($SocialPreviewPath, [System.Drawing.Imaging.ImageFormat]::Png)
}
finally {
    $accentPen.Dispose()
    $subtitleBrush.Dispose()
    $titleBrush.Dispose()
    $backgroundBrush.Dispose()
    $metaFont.Dispose()
    $subtitleFont.Dispose()
    $titleFont.Dispose()
    $iconImage.Dispose()
    $iconStream.Dispose()
    $previewGraphics.Dispose()
    $preview.Dispose()
}
