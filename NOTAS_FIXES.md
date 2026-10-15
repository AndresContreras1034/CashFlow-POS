# Notas de fixes de auditoría

## Fase 2 — Backups y recuperación

- **B07-005:** La rotación debe resolver los archivos desde la carpeta configurada y el nombre validado del backup, sin confiar en la ruta absoluta persistida en `backup_history.file_path`.
  - Prueba: `backup_rotation_path::rotation_ignores_a_manipulated_file_path`.
