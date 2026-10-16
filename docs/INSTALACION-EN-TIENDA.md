# Instalacion de CashFlow POS en la tienda

## Requisitos

- Windows 10 u 11 de 64 bits.
- PostgreSQL 18 instalado y en ejecucion en este equipo, o una instancia PostgreSQL accesible por red.
- Una base de datos vacia llamada `pos_db` y un usuario dedicado para la aplicacion.
- El instalador de CashFlow POS incluido en esta entrega.

## Preparar PostgreSQL

En SQL Shell (psql) o pgAdmin, conectado como administrador de PostgreSQL, crea un usuario y una base de datos. Cambia la contrasena de ejemplo por una contrasena segura:

```sql
CREATE ROLE cashflow_pos LOGIN PASSWORD 'CAMBIAR_POR_UNA_CLAVE_SEGURA';
CREATE DATABASE pos_db OWNER cashflow_pos;
```

No reutilices la contrasena del administrador de PostgreSQL para la aplicacion.

## Instalar y configurar

1. Ejecuta `CashFlow POS_0.1.0_x64-setup.exe` y completa la instalacion.
2. Crea la carpeta `%APPDATA%\com.toonk.pos` si todavia no existe.
3. Dentro de esa carpeta, crea un archivo llamado `.env` (sin extension adicional) con esta linea:

   ```dotenv
   DATABASE_URL=postgres://cashflow_pos:CONTRASENA_URL_ENCODED@localhost:5432/pos_db
   ```

   Reemplaza `CONTRASENA_URL_ENCODED` con la contrasena del usuario PostgreSQL. Si contiene caracteres reservados para URL, codificalos para URL. Si PostgreSQL esta en otro equipo, cambia `localhost` por el nombre o IP de ese servidor.

4. Inicia CashFlow POS. En el primer arranque, la aplicacion crea el esquema ejecutando sus migraciones. El usuario de PostgreSQL debe ser propietario de `pos_db`.
5. Completa la activacion de licencia para ese equipo usando el identificador de instalacion que muestra la aplicacion. Emite la licencia con la herramienta del propietario y transfiere solo el archivo de licencia; nunca copies la clave privada al equipo de la tienda.

## Importante

- No incluyas contrasenas reales, archivos `.env`, claves privadas ni datos de clientes en la entrega.
- Si se usa una base de datos en red, configura PostgreSQL para aceptar conexiones desde el equipo de la tienda y limita el acceso con el firewall.
- Verifica la conexion, la licencia, una venta de prueba y la impresion antes de empezar a operar. Configura tambien las copias de seguridad para la base de datos que se utilizara en produccion.
