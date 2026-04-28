# Firefox and geckodriver are pre-installed on GitHub-hosted Windows runners.
Start-Process -FilePath "$env:GECKOWEBDRIVER\geckodriver.exe" -ArgumentList "--port=4444"
Start-Sleep -Seconds 1
