param(
  [Parameter(Mandatory = $true)]
  [string]$Path
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
  throw "The signing target does not exist."
}

$signToolCommand = Get-Command signtool.exe -ErrorAction SilentlyContinue
if ($null -ne $signToolCommand) {
  $signToolPath = $signToolCommand.Source
}
else {
  $signToolPath = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Filter signtool.exe -Recurse -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending |
    Select-Object -First 1 -ExpandProperty FullName
}

if ([string]::IsNullOrWhiteSpace($signToolPath)) {
  throw "Windows SDK SignTool was not found."
}

& $signToolPath sign /a /fd SHA256 /tr https://timestamp.digicert.com /td SHA256 $Path
if ($LASTEXITCODE -ne 0) {
  throw "SignTool failed to sign the release artifact."
}
