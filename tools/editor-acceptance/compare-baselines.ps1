[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $CandidateRoot,

    [Parameter(Mandatory = $true)]
    [string] $OutputRoot,

    [ValidateRange(0, 255)]
    [int] $PixelThreshold = 24,

    [ValidateRange(0.0, 1.0)]
    [double] $MaxChangedRatio = 0.03,

    [ValidateRange(0.0, 255.0)]
    [double] $MaxMeanChannelDelta = 3.0
)

$ErrorActionPreference = "Stop"
$AcceptanceRoot = Split-Path -Parent $PSCommandPath
$ManifestPath = Join-Path $AcceptanceRoot "baseline-manifest.json"
$CandidateRoot = (Resolve-Path -LiteralPath $CandidateRoot -ErrorAction Stop).Path
$OutputRoot = [IO.Path]::GetFullPath($OutputRoot)

if (-not (Test-Path -LiteralPath $CandidateRoot -PathType Container)) {
    throw "editor visual candidate root is not a directory: $CandidateRoot"
}
if (-not (Test-Path -LiteralPath $ManifestPath -PathType Leaf)) {
    throw "editor visual baseline manifest is missing: $ManifestPath"
}

Add-Type -AssemblyName System.Drawing
if ($null -eq ("EngLang.EditorAcceptance.VisualDiff" -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.IO;
using System.Runtime.InteropServices;

namespace EngLang.EditorAcceptance
{
    public sealed class VisualDiffResult
    {
        public long PixelCount { get; set; }
        public long ChangedPixels { get; set; }
        public double ChangedRatio { get; set; }
        public double MeanChannelDelta { get; set; }
        public int MaxChannelDelta { get; set; }
        public int Width { get; set; }
        public int Height { get; set; }
    }

    public static class VisualDiff
    {
        public static VisualDiffResult Compare(
            string baselinePath,
            string candidatePath,
            string diffPath,
            int pixelThreshold)
        {
            using (var baselineSource = new Bitmap(baselinePath))
            using (var candidateSource = new Bitmap(candidatePath))
            {
                if (baselineSource.Width != candidateSource.Width ||
                    baselineSource.Height != candidateSource.Height)
                {
                    throw new InvalidDataException(String.Format(
                        "image dimensions differ: baseline {0}x{1}, candidate {2}x{3}",
                        baselineSource.Width,
                        baselineSource.Height,
                        candidateSource.Width,
                        candidateSource.Height));
                }

                int width = baselineSource.Width;
                int height = baselineSource.Height;
                using (var baseline = ToArgb(baselineSource))
                using (var candidate = ToArgb(candidateSource))
                using (var diff = new Bitmap(width, height, PixelFormat.Format32bppArgb))
                {
                    var baselinePixels = ReadPixels(baseline);
                    var candidatePixels = ReadPixels(candidate);
                    var diffPixels = new byte[baselinePixels.Length];
                    long changedPixels = 0;
                    long channelDeltaSum = 0;
                    int maxChannelDelta = 0;

                    for (int offset = 0; offset < baselinePixels.Length; offset += 4)
                    {
                        int blueDelta = Math.Abs(baselinePixels[offset] - candidatePixels[offset]);
                        int greenDelta = Math.Abs(baselinePixels[offset + 1] - candidatePixels[offset + 1]);
                        int redDelta = Math.Abs(baselinePixels[offset + 2] - candidatePixels[offset + 2]);
                        int pixelDelta = Math.Max(redDelta, Math.Max(greenDelta, blueDelta));
                        channelDeltaSum += redDelta + greenDelta + blueDelta;
                        maxChannelDelta = Math.Max(maxChannelDelta, pixelDelta);

                        if (pixelDelta > pixelThreshold)
                        {
                            changedPixels += 1;
                            diffPixels[offset] = 255;
                            diffPixels[offset + 1] = 0;
                            diffPixels[offset + 2] = 255;
                        }
                        else
                        {
                            int gray = (baselinePixels[offset] + baselinePixels[offset + 1] + baselinePixels[offset + 2]) / 12;
                            diffPixels[offset] = (byte)gray;
                            diffPixels[offset + 1] = (byte)gray;
                            diffPixels[offset + 2] = (byte)gray;
                        }
                        diffPixels[offset + 3] = 255;
                    }

                    WritePixels(diff, diffPixels);
                    Directory.CreateDirectory(Path.GetDirectoryName(diffPath));
                    diff.Save(diffPath, ImageFormat.Png);

                    long pixelCount = (long)width * height;
                    return new VisualDiffResult
                    {
                        PixelCount = pixelCount,
                        ChangedPixels = changedPixels,
                        ChangedRatio = pixelCount == 0 ? 0.0 : (double)changedPixels / pixelCount,
                        MeanChannelDelta = pixelCount == 0 ? 0.0 : (double)channelDeltaSum / (pixelCount * 3.0),
                        MaxChannelDelta = maxChannelDelta,
                        Width = width,
                        Height = height
                    };
                }
            }
        }

        private static Bitmap ToArgb(Bitmap source)
        {
            var output = new Bitmap(source.Width, source.Height, PixelFormat.Format32bppArgb);
            using (var graphics = Graphics.FromImage(output))
            {
                graphics.CompositingMode = CompositingMode.SourceCopy;
                graphics.DrawImageUnscaled(source, 0, 0);
            }
            return output;
        }

        private static byte[] ReadPixels(Bitmap image)
        {
            var rectangle = new Rectangle(0, 0, image.Width, image.Height);
            var data = image.LockBits(rectangle, ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
            try
            {
                int rowBytes = image.Width * 4;
                var pixels = new byte[rowBytes * image.Height];
                for (int row = 0; row < image.Height; row++)
                {
                    IntPtr rowPointer = IntPtr.Add(data.Scan0, row * data.Stride);
                    Marshal.Copy(rowPointer, pixels, row * rowBytes, rowBytes);
                }
                return pixels;
            }
            finally
            {
                image.UnlockBits(data);
            }
        }

        private static void WritePixels(Bitmap image, byte[] pixels)
        {
            var rectangle = new Rectangle(0, 0, image.Width, image.Height);
            var data = image.LockBits(rectangle, ImageLockMode.WriteOnly, PixelFormat.Format32bppArgb);
            try
            {
                int rowBytes = image.Width * 4;
                for (int row = 0; row < image.Height; row++)
                {
                    IntPtr rowPointer = IntPtr.Add(data.Scan0, row * data.Stride);
                    Marshal.Copy(pixels, row * rowBytes, rowPointer, rowBytes);
                }
            }
            finally
            {
                image.UnlockBits(data);
            }
        }
    }
}
'@ -ReferencedAssemblies System.Drawing
}

$Manifest = Get-Content -LiteralPath $ManifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($Manifest.format -ne "englang-editor-visual-baseline-v1") {
    throw "unsupported editor visual baseline manifest format: $($Manifest.format)"
}

New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
$Comparisons = foreach ($Capture in @($Manifest.captures)) {
    $BaselineRelativePath = ([string]$Capture.path) -replace '/', [IO.Path]::DirectorySeparatorChar
    $BaselinePath = Join-Path $AcceptanceRoot $BaselineRelativePath
    $CandidateName = [IO.Path]::GetFileName([string]$Capture.path)
    $CandidatePath = Join-Path $CandidateRoot $CandidateName
    $DiffName = [IO.Path]::GetFileNameWithoutExtension($CandidateName) + "-diff.png"
    $DiffPath = Join-Path $OutputRoot $DiffName

    if (-not (Test-Path -LiteralPath $CandidatePath -PathType Leaf)) {
        [pscustomobject]@{
            capture = $CandidateName
            passed = $false
            error = "candidate image is missing"
        }
        continue
    }

    try {
        $Result = [EngLang.EditorAcceptance.VisualDiff]::Compare(
            $BaselinePath,
            $CandidatePath,
            $DiffPath,
            $PixelThreshold
        )
        $Passed = $Result.ChangedRatio -le $MaxChangedRatio -and
            $Result.MeanChannelDelta -le $MaxMeanChannelDelta
        [pscustomobject]@{
            capture = $CandidateName
            passed = $Passed
            width = $Result.Width
            height = $Result.Height
            changed_pixels = $Result.ChangedPixels
            pixel_count = $Result.PixelCount
            changed_ratio = [Math]::Round($Result.ChangedRatio, 8)
            mean_channel_delta = [Math]::Round($Result.MeanChannelDelta, 4)
            max_channel_delta = $Result.MaxChannelDelta
            diff = $DiffName
            error = $null
        }
    } catch {
        [pscustomobject]@{
            capture = $CandidateName
            passed = $false
            error = $_.Exception.Message
        }
    }
}

$Summary = [ordered]@{
    format = "englang-editor-visual-diff-v1"
    baseline_manifest = "tools/editor-acceptance/baseline-manifest.json"
    candidate_root = $CandidateRoot
    thresholds = [ordered]@{
        significant_channel_delta = $PixelThreshold
        max_changed_ratio = $MaxChangedRatio
        max_mean_channel_delta = $MaxMeanChannelDelta
    }
    comparisons = @($Comparisons)
}
$SummaryPath = Join-Path $OutputRoot "visual-diff-summary.json"
$SummaryJson = $Summary | ConvertTo-Json -Depth 8
[IO.File]::WriteAllText($SummaryPath, $SummaryJson + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))

foreach ($Comparison in $Comparisons) {
    if ($Comparison.passed) {
        Write-Host ("ok: {0} changed={1:P3} mean_delta={2:N4}" -f $Comparison.capture, $Comparison.changed_ratio, $Comparison.mean_channel_delta)
    } else {
        $FailureDetail = if ($null -ne $Comparison.error) {
            $Comparison.error
        } else {
            "changed=$($Comparison.changed_ratio.ToString('P3')) mean_delta=$($Comparison.mean_channel_delta.ToString('N4'))"
        }
        Write-Host ("failed: {0} {1}" -f $Comparison.capture, $FailureDetail)
    }
}

$Failures = @($Comparisons | Where-Object { -not $_.passed })
if ($Failures.Count -gt 0) {
    throw "editor visual comparison failed for $($Failures.Count) capture(s); inspect $SummaryPath and the generated diff PNG files"
}

Write-Host "Editor visual comparison passed. Summary: $SummaryPath"
