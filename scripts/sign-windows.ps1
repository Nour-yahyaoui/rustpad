<#
.SYNOPSIS
    Authenticode Code-Signing Helper for RustPad Desktop Studio (`rustpad.exe`).

.DESCRIPTION
    Embeds an X.509 Authenticode publisher signature ("Nour Yahyaoui - RustPad Studio")
    into `rustpad.exe` using native Windows PowerShell (`New-SelfSignedCertificate` +
    `Set-AuthenticodeSignature`) with zero external SDK tools required.

    - For local developer machines or enterprise IT fleets: pass `-TrustRoot` (as Admin)
      or deploy the exported `.cer` via Group Policy / Intune Trusted Publishers so
      Windows SmartScreen & Defender Application Control recognize the publisher immediately.
    - For public CA / EV / Azure Trusted Signing certificates: pass `-CertThumbprint <SHA1>`
      to sign with an existing CodeSigning certificate in `Cert:\CurrentUser\My`.

.EXAMPLE
    pwsh -ExecutionPolicy Bypass -File scripts/sign-windows.ps1 -ExePath dist/rustpad.exe
#>
param(
    [Parameter(Mandatory = $false)]
    [string]$ExePath = "target\release\rustpad.exe",

    [Parameter(Mandatory = $false)]
    [string]$PublisherSubject = "CN=Nour Yahyaoui - RustPad Studio, O=RustPad Open-Source Project, C=TN",

    [Parameter(Mandatory = $false)]
    [string]$CertThumbprint = "",

    [Parameter(Mandatory = $false)]
    [string]$TimestampServer = "http://timestamp.digicert.com",

    [Parameter(Mandatory = $false)]
    [switch]$TrustRoot
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $ExePath)) {
    throw "Target executable not found at '$ExePath'. Run 'cargo build --release -p rustpad' first."
}

$cert = $null
if ($CertThumbprint -ne "") {
    $cert = Get-ChildItem -Path "Cert:\CurrentUser\My\$CertThumbprint" -ErrorAction Stop
} else {
    $existing = Get-ChildItem -Path "Cert:\CurrentUser\My" -CodeSigningCert |
        Where-Object { $_.Subject -eq $PublisherSubject -and $_.NotAfter -gt (Get-Date) } |
        Select-Object -First 1

    if ($null -ne $existing) {
        $cert = $existing
        Write-Host "[RustPad Sign] Reusing existing CodeSigning certificate: $($cert.Thumbprint)"
    } else {
        Write-Host "[RustPad Sign] Creating CodeSigning certificate for '$PublisherSubject'..."
        $cert = New-SelfSignedCertificate `
            -Type CodeSigningCert `
            -Subject $PublisherSubject `
            -KeyAlgorithm RSA `
            -KeyLength 3072 `
            -HashAlgorithm SHA256 `
            -CertStoreLocation "Cert:\CurrentUser\My" `
            -NotAfter (Get-Date).AddYears(3)
    }
}

if ($TrustRoot) {
    $cerPath = Join-Path ([System.IO.Path]::GetTempPath()) "rustpad-publisher.cer"
    Export-Certificate -Cert $cert -FilePath $cerPath -Force | Out-Null
    Import-Certificate -FilePath $cerPath -CertStoreLocation "Cert:\CurrentUser\Root" | Out-Null
    Import-Certificate -FilePath $cerPath -CertStoreLocation "Cert:\CurrentUser\TrustedPublisher" | Out-Null
    Write-Host "[RustPad Sign] Installed publisher certificate into CurrentUser Root & TrustedPublisher."
}

Write-Host "[RustPad Sign] Applying SHA-256 Authenticode signature to '$ExePath'..."
try {
    $sig = Set-AuthenticodeSignature `
        -FilePath $ExePath `
        -Certificate $cert `
        -HashAlgorithm SHA256 `
        -TimestampServer $TimestampServer
} catch {
    # Offline / air-gapped fallback without RFC 3161 network timestamp
    $sig = Set-AuthenticodeSignature `
        -FilePath $ExePath `
        -Certificate $cert `
        -HashAlgorithm SHA256
}

Write-Host "[RustPad Sign] Completed -> Status: $($sig.Status) | Signer: $($sig.SignerCertificate.Subject)"
