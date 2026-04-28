# Chrome and chromedriver are pre-installed on GitHub-hosted Windows runners.
Start-Process -FilePath "$env:CHROMEWEBDRIVER\chromedriver.exe" -ArgumentList "--port=9515"
Start-Sleep -Seconds 1
