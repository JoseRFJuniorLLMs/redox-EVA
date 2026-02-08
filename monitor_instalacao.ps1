# Monitor de Instalacao do Build Tools e Compilacao do Driver
# Executa em loop ate Build Tools estar pronto

Write-Host "`n╔══════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║  Monitor: VS Build Tools + Driver NPU           ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════╝`n" -ForegroundColor Cyan

$buildToolsPath = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC"

while ($true) {
    $timestamp = Get-Date -Format "HH:mm:ss"

    # Verificar se Build Tools esta instalado
    if (Test-Path $buildToolsPath) {
        Write-Host "[$timestamp] Build Tools instalado!" -ForegroundColor Green

        # Configurar ambiente MSVC
        $vcvarsPath = Get-ChildItem -Path $buildToolsPath -Recurse -Filter "vcvars64.bat" | Select-Object -First 1

        if ($vcvarsPath) {
            Write-Host "[$timestamp] Compilando driver NPU..." -ForegroundColor Yellow

            # Trocar para toolchain MSVC
            & "$env:USERPROFILE\.cargo\bin\rustup.exe" default nightly-x86_64-pc-windows-msvc

            # Compilar driver
            Push-Location "d:\DEV\EVA-OS\drive"
            & "$env:USERPROFILE\.cargo\bin\cargo.exe" clean
            & "$env:USERPROFILE\.cargo\bin\cargo.exe" build --release
            $exitCode = $LASTEXITCODE
            Pop-Location

            if ($exitCode -eq 0) {
                Write-Host "`n[$timestamp] DRIVER COMPILADO COM SUCESSO!" -ForegroundColor Green
                Write-Host "[$timestamp] Binario: d:\DEV\EVA-OS\drive\target\release\intel-npu.exe" -ForegroundColor Cyan

                # Executar teste
                Write-Host "`n[$timestamp] Executando teste de PCI discovery..." -ForegroundColor Yellow
                & "d:\DEV\EVA-OS\drive\target\release\intel-npu.exe" --test
            } else {
                Write-Host "`n[$timestamp] Erro na compilacao (exit code: $exitCode)" -ForegroundColor Red
            }
        }

        break
    }

    # Verificar progresso da instalacao
    $processes = Get-Process | Where-Object { $_.Name -like "*vs_*" -or $_.Name -like "*setup*" }
    if ($processes) {
        Write-Host "[$timestamp] Instalando Build Tools... ($($processes.Count) processos ativos)" -ForegroundColor Yellow
    } else {
        Write-Host "[$timestamp] Aguardando instalacao iniciar..." -ForegroundColor Gray
    }

    # Aguardar 30 segundos
    Start-Sleep -Seconds 30
}

Write-Host "`n[$timestamp] Monitor finalizado." -ForegroundColor Green
