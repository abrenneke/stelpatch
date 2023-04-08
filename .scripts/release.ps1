# Parse command line arguments
$exeName = $args[0]
$versionIndex = [array]::IndexOf($args, '--version')
if ($versionIndex -ge 0 -and $versionIndex -lt ($args.Length - 1)) {
    $releaseVersion = $args[$versionIndex + 1]
} else {
    $releaseVersion = "0.0.0" # Default version number
}

# Set other variables
$targetDir = "target/release"
$releaseDir = ".release/$exeName"
$releaseExecutable = "$exeName-$releaseVersion.exe"

# Build release binaries
cargo build --release

# Create release directory if it doesn't exist
if (!(Test-Path $releaseDir)) {
    New-Item -ItemType Directory -Path $releaseDir | Out-Null
}

# Copy specified executable file to release directory
Copy-Item -Path "$targetDir/$exeName.exe" -Destination "$releaseDir/$releaseExecutable" -Force


# Print success message
Write-Host "Release executable created: $releaseExecutable"
