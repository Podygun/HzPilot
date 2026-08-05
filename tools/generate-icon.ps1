param(
    [string]$OutputPath = "$PSScriptRoot\..\assets\hzpilot.ico"
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
