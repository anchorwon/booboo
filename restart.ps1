# Find process listening on port 1420 (Vite)
$tcps = Get-NetTCPConnection -LocalPort 1420 -ErrorAction SilentlyContinue
if ($tcps) {
    foreach ($tcp in $tcps) {
        $procId = $tcp.OwningProcess
        echo "Killing process on port 1420 (PID: $procId)"
        Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
    }
} else {
    echo "No process found on port 1420."
}

# Kill any lingering tauri-app.exe
echo "Killing existing tauri-app.exe processes..."
Get-Process tauri-app -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500
echo "All tauri-app.exe processes killed."

echo "Starting Tauri app..."
$env:RUST_BACKTRACE = "1"
npm run tauri dev
