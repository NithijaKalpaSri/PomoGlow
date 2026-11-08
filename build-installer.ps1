# Builds the app, packs it into the tiny installer, and copies both to dist/ and web/public/downloads.
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
cargo build --release
python -c "import zlib;d=open('target/release/pomoglow.exe','rb').read();open('installer/app.z','wb').write(zlib.compress(d,9))"
Push-Location installer; cargo build --release; Pop-Location
New-Item -ItemType Directory -Force dist, web/public/downloads | Out-Null
Copy-Item installer/target/release/pomoglow-setup.exe dist/Pomoglow-Setup.exe -Force
Copy-Item installer/target/release/pomoglow-setup.exe web/public/downloads/Pomoglow-Setup.exe -Force
Copy-Item target/release/pomoglow.exe web/public/downloads/pomoglow.exe -Force
Get-ChildItem dist | Select-Object Name, @{n='MB';e={[math]::Round($_.Length/1MB,2)}}
