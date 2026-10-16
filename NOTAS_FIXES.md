# Notas de fixes de auditoría

## Fase 2 — Backups y recuperación

- **B07-005:** La rotación debe resolver los archivos desde la carpeta configurada y el nombre validado del backup, sin confiar en la ruta absoluta persistida en `backup_history.file_path`.
  - Prueba: `backup_rotation_path::rotation_ignores_a_manipulated_file_path`.
- **B07-002:** Las bases temporales de restore/inspección llevan marca de tiempo y sufijo aleatorio; se limpian al inicio y antes de restaurar solo si tienen más de una hora y no están en uso. Los residuos heredados sin fecha verificable se marcan al detectarse, se conservan por al menos una hora y generan un aviso. Si falta la base activa, se recupera desde el respaldo `pre_restore` más reciente y se muestra un aviso.
  - Pruebas: `restore::tests::stale_scratch_databases_are_removed_but_recent_ones_are_preserved`, `restore::tests::missing_live_database_is_recovered_from_pre_restore` y `restore::tests::failed_restore_rename_rolls_back_the_live_database`.
- **B07-008:** Los temporales de backup/restauración usan `%LOCALAPPDATA%\CashFlow\tmp` en Windows, prefijos distintos y limpieza de archivos con más de una hora al iniciar y antes de cada operación.
  - Pruebas: `backup::temp::tests::stale_cleanup_requires_age_and_a_known_prefix` y `backup::temp::tests::cleanup_preserves_recent_and_unrelated_temporaries`.
- **B07-001:** El cifrado queda habilitado por migración; el banner persistente guía la configuración si el backup automático no está cifrado o falta la contraseña. Desactivarlo requiere confirmación y se advierte si el destino parece ser OneDrive.
  - Pruebas: `backup_encryption_default::backup_encryption_is_enabled_by_default_for_existing_and_new_rows` y utilidades `needsBackupEncryptionSetup` / `isOneDrivePath`.
