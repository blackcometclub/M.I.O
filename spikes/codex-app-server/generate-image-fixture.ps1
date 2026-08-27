param(
    [string]$OutputPath = (Join-Path $PSScriptRoot 'fixtures\local-image-vision.png')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing

$outputDirectory = Split-Path -Parent $OutputPath
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null

$image = [System.Drawing.Bitmap]::new(1200, 800)
$graphics = [System.Drawing.Graphics]::FromImage($image)
$titleFont = [System.Drawing.Font]::new('Segoe UI', 46, [System.Drawing.FontStyle]::Bold)
$shapeFont = [System.Drawing.Font]::new('Segoe UI', 110, [System.Drawing.FontStyle]::Bold)
$codeFont = [System.Drawing.Font]::new('Consolas', 50, [System.Drawing.FontStyle]::Bold)
$smallFont = [System.Drawing.Font]::new('Segoe UI', 24, [System.Drawing.FontStyle]::Regular)
$whiteBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
$navyBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 22, 30, 58))
$cyanBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 33, 210, 220))
$yellowBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 255, 211, 64))
$pinkBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 255, 91, 155))
$darkBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 28, 28, 35))
$panelBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(255, 49, 61, 101))

try {
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $graphics.FillRectangle($navyBrush, 0, 0, 1200, 800)

    $graphics.DrawString('M.O.E. VISION FIXTURE', $titleFont, $whiteBrush, 62, 42)
    $graphics.DrawString('Read the image, not the prompt.', $smallFont, $whiteBrush, 68, 125)

    $graphics.FillEllipse($cyanBrush, 80, 220, 280, 280)
    $graphics.DrawString('7', $shapeFont, $darkBrush, 178, 270)

    $graphics.FillRectangle($yellowBrush, 460, 220, 280, 280)
    $graphics.DrawString([char]0x7F8A, $shapeFont, $darkBrush, 500, 270)

    $triangle = [System.Drawing.Point[]]@(
        [System.Drawing.Point]::new(850, 500),
        [System.Drawing.Point]::new(1000, 220),
        [System.Drawing.Point]::new(1150, 500)
    )
    $graphics.FillPolygon($pinkBrush, $triangle)

    $graphics.FillRectangle($panelBrush, 60, 590, 1080, 145)
    $graphics.DrawString('CODE: NEKOMIMI-42', $codeFont, $whiteBrush, 132, 625)

    $image.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
}
finally {
    $panelBrush.Dispose()
    $darkBrush.Dispose()
    $pinkBrush.Dispose()
    $yellowBrush.Dispose()
    $cyanBrush.Dispose()
    $navyBrush.Dispose()
    $whiteBrush.Dispose()
    $smallFont.Dispose()
    $codeFont.Dispose()
    $shapeFont.Dispose()
    $titleFont.Dispose()
    $graphics.Dispose()
    $image.Dispose()
}

Write-Output (Resolve-Path $OutputPath)
