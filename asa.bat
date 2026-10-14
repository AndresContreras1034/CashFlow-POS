@echo off
setlocal

rem ===== BACKEND =====
set B=src-tauri\src\modules\backup
mkdir "%B%\destinations" 2>nul
mkdir "src-tauri\tests" 2>nul
mkdir "src-tauri\migrations" 2>nul

for %%F in (mod dto models repository service handlers router scheduler pg_tools crypto metadata retention restore status) do (
  if not exist "%B%\%%F.rs" type nul > "%B%\%%F.rs"
)
for %%F in (mod local smtp cloud) do (
  if not exist "%B%\destinations\%%F.rs" type nul > "%B%\destinations\%%F.rs"
)
for %%F in (backup_service backup_retention backup_restore) do (
  if not exist "src-tauri\tests\%%F.rs" type nul > "src-tauri\tests\%%F.rs"
)
if not exist "src-tauri\migrations\20261013000000_create_backup_tables.sql" type nul > "src-tauri\migrations\20261013000000_create_backup_tables.sql"

rem ===== FRONTEND =====
set C=src\components\backup
mkdir "%C%" 2>nul
mkdir "src\pages\RestoreBackup" 2>nul

for %%F in (BackupSettingsForm BackupRecipients BackupHistoryList BackupStatusBadge BackupAlertBanner RestoreModal) do (
  if not exist "%C%\%%F.tsx" type nul > "%C%\%%F.tsx"
  if not exist "%C%\%%F.css" type nul > "%C%\%%F.css"
)

if not exist "src\pages\RestoreBackup\RestoreBackup.tsx" type nul > "src\pages\RestoreBackup\RestoreBackup.tsx"
if not exist "src\pages\RestoreBackup\RestoreBackup.css" type nul > "src\pages\RestoreBackup\RestoreBackup.css"
if not exist "src\types\backup.ts" type nul > "src\types\backup.ts"
if not exist "src\services\backup.service.ts" type nul > "src\services\backup.service.ts"
if not exist "src\hooks\useBackupStatus.ts" type nul > "src\hooks\useBackupStatus.ts"
if not exist "src\utils\backup.ts" type nul > "src\utils\backup.ts"
if not exist "src\utils\backup.test.ts" type nul > "src\utils\backup.test.ts"

echo Estructura de backup creada.
endlocal