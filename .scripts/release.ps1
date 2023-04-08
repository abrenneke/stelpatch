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
$zipDir = ".release"
$zipFileName = "$exeName-$releaseVersion.zip"

# Build release binaries
cargo build --release

# Create release directory if it doesn't exist
if (!(Test-Path $releaseDir)) {
    New-Item -ItemType Directory -Path $releaseDir | Out-Null
}

# Copy specified executable file and dependencies to release directory
Copy-Item -Path "$targetDir/$exeName.exe" -Destination $releaseDir -Force
Copy-Item -Path "$targetDir/*.dll" -Destination $releaseDir -Force

# Zip up release directory
Compress-Archive -Path $releaseDir/* -DestinationPath "$zipDir/$zipFileName" -Force

# Clean up release directory
Remove-Item $releaseDir -Recurse
