Add-Type -AssemblyName System.Windows.Forms

$setup = Join-Path $PSScriptRoot 'com.susenayw.amplifi-win-Setup.exe'
if (-not (Test-Path -LiteralPath $setup -PathType Leaf)) {
    [System.Windows.Forms.MessageBox]::Show(
        "Place com.susenayw.amplifi-win-Setup.exe in this folder, then run this launcher again.",
        'Amplifi installer',
        [System.Windows.Forms.MessageBoxButtons]::OK,
        [System.Windows.Forms.MessageBoxIcon]::Error
    ) | Out-Null
    exit 1
}

$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.Description = 'Choose the folder where Amplifi will be installed.'
$dialog.ShowNewFolderButton = $true
$dialog.SelectedPath = $env:LOCALAPPDATA

if ($dialog.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) {
    exit 0
}

try {
    $arguments = '--installto "{0}"' -f $dialog.SelectedPath
    $process = Start-Process -FilePath $setup -ArgumentList $arguments -Wait -PassThru
    exit $process.ExitCode
} catch {
    [System.Windows.Forms.MessageBox]::Show(
        $_.Exception.Message,
        'Amplifi installation failed',
        [System.Windows.Forms.MessageBoxButtons]::OK,
        [System.Windows.Forms.MessageBoxIcon]::Error
    ) | Out-Null
    exit 1
}
