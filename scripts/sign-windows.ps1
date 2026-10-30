<#
.SYNOPSIS
    Authenticode Code-Signing Helper for RustPad Desktop Studio (`rustpad.exe`).
    Supports `.pem` (+ optional `.key`), `.pfx`, Cloud HSM / USB Token (`Cert:\CurrentUser\My`),
    or local self-signed developer certificates.

.EXAMPLE
    # 1. Sign using a single combined .pem file (contains both CERTIFICATE and PRIVATE KEY):
    powershell -ExecutionPolicy Bypass -File scripts/sign-windows.ps1 -ExePath .\rustpad.exe -PemPath .\cert.pem

.EXAMPLE
    # 2. Sign using a public .pem certificate + separate private key file:
    powershell -ExecutionPolicy Bypass -File scripts/sign-windows.ps1 -ExePath .\rustpad.exe -PemPath .\cert.pem -KeyPath .\private.key

.EXAMPLE
    # 3. Sign using a .pfx / .p12 bundle:
    powershell -ExecutionPolicy Bypass -File scripts/sign-windows.ps1 -ExePath .\rustpad.exe -PfxPath .\cert.pfx

.EXAMPLE
    # 4. Sign using a Cloud HSM / USB Token (e.g., Certum SimplySign, DigiCert, YubiKey):
    powershell -ExecutionPolicy Bypass -File scripts/sign-windows.ps1 -ExePath .\rustpad.exe -PemPath .\cert.pem
#>
param(
    [Parameter(Mandatory = $false)]
    [string]$ExePath = "target\release\rustpad.exe",

    [Parameter(Mandatory = $false)]
    [string]$PemPath = "",

    [Parameter(Mandatory = $false)]
    [string]$KeyPath = "",

    [Parameter(Mandatory = $false)]
    [string]$PfxPath = "",

    [Parameter(Mandatory = $false)]
    [string]$CertThumbprint = "",

    [Parameter(Mandatory = $false)]
    [string]$PublisherSubject = "CN=Nour Yahyaoui - RustPad Studio, O=RustPad Open-Source Project, C=TN",

    [Parameter(Mandatory = $false)]
    [string]$TimestampServer = "http://timestamp.digicert.com",

    [Parameter(Mandatory = $false)]
    [switch]$TrustRoot
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path -LiteralPath $ExePath)) {
    throw "Target executable not found at '$ExePath'. Pass -ExePath .\path\to\rustpad.exe"
}

$cert = $null

if ($PemPath -ne "") {
    if (-not (Test-Path -LiteralPath $PemPath)) {
        throw "PEM file not found at '$PemPath'."
    }
    $pemText = Get-Content -LiteralPath $PemPath -Raw
    $hasCert = $pemText -match "-----BEGIN CERTIFICATE-----"
    $hasInlineKey = $pemText -match "PRIVATE KEY-----"

    if (-not $hasCert) {
        throw "The file '$PemPath' does not contain a '-----BEGIN CERTIFICATE-----' block. If '$PemPath' is your private key, pass your public certificate as -PemPath and the key as -KeyPath."
    }

    if ($hasInlineKey -or $KeyPath -ne "") {
        $effectiveKeyPath = if ($KeyPath -ne "") { $KeyPath } else { $PemPath }
        if (-not (Test-Path -LiteralPath $effectiveKeyPath)) {
            throw "Private key file not found at '$effectiveKeyPath'."
        }

        Write-Host "[RustPad Sign] Loading certificate + private key from PEM..."
        # PowerShell 7+ (.NET 5+) native PEM loader
        if ([System.Security.Cryptography.X509Certificates.X509Certificate2].GetMethod("CreateFromPemFile", [type[]]@([string], [string]))) {
            $ephemeral = [System.Security.Cryptography.X509Certificates.X509Certificate2]::CreateFromPemFile($PemPath, $effectiveKeyPath)
            # Round-trip through ephemeral PKCS#12 bytes so Windows CAPI/CNG can sign Authenticode
            $pfxBytes = $ephemeral.Export([System.Security.Cryptography.X509Certificates.X509ContentType]::Pfx)
            $cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2(
                $pfxBytes,
                "",
                [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::Exportable -bor [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::PersistKeySet
            )
        } else {
            # Windows PowerShell 5.1 fallback via openssl or certutil -mergepfx
            $tmpPfx = Join-Path ([System.IO.Path]::GetTempPath()) ("rustpad-tmp-" + [guid]::NewGuid().ToString("N") + ".pfx")
            try {
                if (Get-Command openssl -ErrorAction SilentlyContinue) {
                    & openssl pkcs12 -export -out $tmpPfx -in $PemPath -inkey $effectiveKeyPath -passout pass: | Out-Null
                } else {
                    $tmpCer = [System.IO.Path]::ChangeExtension($tmpPfx, ".cer")
                    $tmpKey = [System.IO.Path]::ChangeExtension($tmpPfx, ".key")
                    Copy-Item -LiteralPath $PemPath -Destination $tmpCer -Force
                    Copy-Item -LiteralPath $effectiveKeyPath -Destination $tmpKey -Force
                    & certutil -mergepfx -p "" $tmpCer $tmpPfx | Out-Null
                    Remove-Item -LiteralPath $tmpCer, $tmpKey -Force -ErrorAction SilentlyContinue
                }
                $cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2(
                    $tmpPfx,
                    "",
                    [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::PersistKeySet
                )
            } finally {
                Remove-Item -LiteralPath $tmpPfx -Force -ErrorAction SilentlyContinue
            }
        }
    } else {
        # Public-only .pem downloaded from a CA (e.g. Certum SimplySign, SSL.com, DigiCert, or Windows CSR)
        Write-Host "[RustPad Sign] '$PemPath' contains a public certificate (no inline private key)."
        Write-Host "[RustPad Sign] Checking Windows Certificate Store / Cloud HSM (SimplySign / USB Token) for matching key..."
        $pubCert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2($PemPath)
        $thumb = $pubCert.Thumbprint

        # Attempt to import & bind to any pending CSR key or SimplySign/HSM provider in CurrentUser\My
        & certutil -user -addstore My $PemPath | Out-Null
        & certutil -user -repairstore My $thumb | Out-Null

        $matched = Get-ChildItem -Path "Cert:\CurrentUser\My\$thumb" -ErrorAction SilentlyContinue
        if ($null -ne $matched -and $matched.HasPrivateKey) {
            $cert = $matched
            Write-Host "[RustPad Sign] Bound public PEM to private key in Cert:\CurrentUser\My\$thumb!"
        } else {
            throw @"
'$PemPath' only contains your PUBLIC certificate (-----BEGIN CERTIFICATE-----), not the private key.
To sign rustpad.exe, do ONE of the following:
  1. If you also have a private key file (e.g. private.key or key.pem), pass BOTH:
       -PemPath '$PemPath' -KeyPath .\private.key
  2. If your CA uses a Cloud HSM app (like Certum SimplySign Desktop) or USB hardware token:
       Open & log in to SimplySign Desktop (or plug in the USB token), then re-run this command.
"@
        }
    }
} elseif ($PfxPath -ne "") {
    if (-not (Test-Path -LiteralPath $PfxPath)) {
        throw "PFX file not found at '$PfxPath'."
    }
    $pwd = Read-Host "Enter PFX password (press Enter if none)" -AsSecureString
    $cert = Get-PfxCertificate -FilePath $PfxPath -Password $pwd
} elseif ($CertThumbprint -ne "") {
    $cert = Get-ChildItem -Path "Cert:\CurrentUser\My\$CertThumbprint" -ErrorAction Stop
} else {
    $existing = Get-ChildItem -Path "Cert:\CurrentUser\My" -CodeSigningCert |
        Where-Object { $_.Subject -eq $PublisherSubject -and $_.NotAfter -gt (Get-Date) } |
        Select-Object -First 1

    if ($null -ne $existing) {
        $cert = $existing
        Write-Host "[RustPad Sign] Reusing existing CodeSigning certificate: $($cert.Thumbprint)"
    } else {
        Write-Host "[RustPad Sign] Creating local CodeSigning certificate for '$PublisherSubject'..."
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
        -TimestampServer $TimestampServer `
        -IncludeChain All
} catch {
    # Offline / air-gapped fallback without RFC 3161 network timestamp
    $sig = Set-AuthenticodeSignature `
        -FilePath $ExePath `
        -Certificate $cert `
        -HashAlgorithm SHA256 `
        -IncludeChain All
}

if ($sig.Status -ne "Valid" -and $sig.Status -ne "UnknownError") {
    Write-Warning "[RustPad Sign] Signature status: $($sig.Status) ($($sig.StatusMessage))"
}
Write-Host "[RustPad Sign] Completed -> Status: $($sig.Status) | Publisher: $($sig.SignerCertificate.Subject)"
