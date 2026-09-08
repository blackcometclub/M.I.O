[CmdletBinding()]
param(
    [string]$OutputDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repositoryRoot "docs\assets\storefronts"
}
elseif (-not [System.IO.Path]::IsPathRooted($OutputDirectory)) {
    $OutputDirectory = Join-Path $repositoryRoot $OutputDirectory
}

[void](New-Item -ItemType Directory -Path $OutputDirectory -Force)

$screenshotPath = Join-Path $repositoryRoot "docs\assets\screenshots\mio-talk-room.png"
$iconPath = Join-Path $repositoryRoot "apps\desktop\src-tauri\icons\icon.png"
foreach ($requiredPath in @($screenshotPath, $iconPath)) {
    if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
        throw "Required storefront image source is missing: $requiredPath"
    }
}

function New-RoundedRectanglePath {
    param(
        [System.Drawing.RectangleF]$Rectangle,
        [float]$Radius
    )

    $diameter = $Radius * 2
    $path = [System.Drawing.Drawing2D.GraphicsPath]::new()
    $path.AddArc($Rectangle.X, $Rectangle.Y, $diameter, $diameter, 180, 90)
    $path.AddArc($Rectangle.Right - $diameter, $Rectangle.Y, $diameter, $diameter, 270, 90)
    $path.AddArc($Rectangle.Right - $diameter, $Rectangle.Bottom - $diameter, $diameter, $diameter, 0, 90)
    $path.AddArc($Rectangle.X, $Rectangle.Bottom - $diameter, $diameter, $diameter, 90, 90)
    $path.CloseFigure()
    return $path
}

function Draw-RoundedImage {
    param(
        [System.Drawing.Graphics]$Graphics,
        [System.Drawing.Image]$Image,
        [System.Drawing.RectangleF]$Destination,
        [float]$Radius
    )

    $path = New-RoundedRectanglePath -Rectangle $Destination -Radius $Radius
    $previousClip = $Graphics.Clip
    try {
        $Graphics.SetClip($path)
        $sourceRatio = $Image.Width / $Image.Height
        $destinationRatio = $Destination.Width / $Destination.Height
        if ($sourceRatio -gt $destinationRatio) {
            $sourceHeight = $Image.Height
            $sourceWidth = [int]($sourceHeight * $destinationRatio)
            $sourceX = [int](($Image.Width - $sourceWidth) / 2)
            $sourceY = 0
        }
        else {
            $sourceWidth = $Image.Width
            $sourceHeight = [int]($sourceWidth / $destinationRatio)
            $sourceX = 0
            $sourceY = [int](($Image.Height - $sourceHeight) / 2)
        }
        $source = [System.Drawing.Rectangle]::new($sourceX, $sourceY, $sourceWidth, $sourceHeight)
        $Graphics.DrawImage($Image, $Destination, $source, [System.Drawing.GraphicsUnit]::Pixel)
    }
    finally {
        $Graphics.Clip = $previousClip
        $path.Dispose()
    }
}

function Draw-Cover {
    param(
        [int]$Width,
        [int]$Height,
        [string]$Tagline,
        [string]$Detail,
        [float]$ImageTopRatio,
        [string]$OutputPath
    )

    $canvas = [System.Drawing.Bitmap]::new($Width, $Height)
    $graphics = [System.Drawing.Graphics]::FromImage($canvas)
    $screenshot = [System.Drawing.Image]::FromFile($screenshotPath)
    $icon = [System.Drawing.Image]::FromFile($iconPath)
    $titleFont = [System.Drawing.Font]::new("Segoe UI", [float]($Width * 0.075), [System.Drawing.FontStyle]::Bold)
    $taglineFont = [System.Drawing.Font]::new("Yu Gothic UI", [float]($Width * 0.031), [System.Drawing.FontStyle]::Bold)
    $detailFont = [System.Drawing.Font]::new("Yu Gothic UI", [float]($Width * 0.018), [System.Drawing.FontStyle]::Regular)
    $brandBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(164, 44, 34))
    $textBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(49, 38, 28))
    $detailBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(87, 64, 39))
    $borderPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(255, 247, 226), [float]($Width * 0.008))
    try {
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

        $background = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
            [System.Drawing.Rectangle]::new(0, 0, $Width, $Height),
            [System.Drawing.Color]::FromArgb(255, 198, 56),
            [System.Drawing.Color]::FromArgb(255, 237, 170),
            35.0
        )
        try {
            $graphics.FillRectangle($background, 0, 0, $Width, $Height)
        }
        finally {
            $background.Dispose()
        }

        $margin = [float]($Width * 0.065)
        $iconSize = [float]($Width * 0.12)
        $graphics.DrawImage($icon, $margin, $margin, $iconSize, $iconSize)
        $graphics.DrawString("M.I.O.", $titleFont, $brandBrush, $margin + $iconSize + ($Width * 0.025), $margin - ($Height * 0.008))
        $graphics.DrawString($Tagline, $taglineFont, $textBrush, $margin, $margin + $iconSize + ($Height * 0.02))
        $graphics.DrawString($Detail, $detailFont, $detailBrush, $margin, $margin + $iconSize + ($Height * 0.075))

        $imageTop = [float]($Height * $ImageTopRatio)
        $imageRect = [System.Drawing.RectangleF]::new(
            $margin,
            $imageTop,
            $Width - ($margin * 2),
            $Height - $imageTop - $margin
        )
        Draw-RoundedImage -Graphics $graphics -Image $screenshot -Destination $imageRect -Radius ([float]($Width * 0.025))
        $imagePath = New-RoundedRectanglePath -Rectangle $imageRect -Radius ([float]($Width * 0.025))
        try {
            $graphics.DrawPath($borderPen, $imagePath)
        }
        finally {
            $imagePath.Dispose()
        }

        $canvas.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)
    }
    finally {
        $borderPen.Dispose()
        $detailBrush.Dispose()
        $textBrush.Dispose()
        $brandBrush.Dispose()
        $detailFont.Dispose()
        $taglineFont.Dispose()
        $titleFont.Dispose()
        $icon.Dispose()
        $screenshot.Dispose()
        $graphics.Dispose()
        $canvas.Dispose()
    }
}

$boothPath = Join-Path $OutputDirectory "mio-booth-cover-1200x1200.png"
$itchPath = Join-Path $OutputDirectory "mio-itch-cover-1260x1000.png"

Draw-Cover `
    -Width 1200 `
    -Height 1200 `
    -Tagline "複数AIを、ひとつのTalk Roomへ。" `
    -Detail "Windows 11  |  Codex  |  Claude Code  |  Gemini  |  Grok" `
    -ImageTopRatio 0.34 `
    -OutputPath $boothPath

Draw-Cover `
    -Width 1260 `
    -Height 1000 `
    -Tagline "Multiple AIs. One persistent Talk Room." `
    -Detail "Windows 11  |  Direct mode  |  Codex Conductor" `
    -ImageTopRatio 0.40 `
    -OutputPath $itchPath

Get-Item -LiteralPath $boothPath, $itchPath |
    Select-Object FullName, Length, LastWriteTime
