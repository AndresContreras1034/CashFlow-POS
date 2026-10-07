# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## Licencias offline

La aplicación acepta una licencia de compra perpetua o de alquiler, emitida para el
identificador de una instalación. En la pantalla de activación, copia ese identificador
y úsalo para emitir un archivo JSON:

```powershell
$installationId = "UUID-copiado-de-la-aplicacion"
$privateKey = "$env:APPDATA\CashFlowPOS\license-issuer\license-private.pem"
node scripts/license-issuer.mjs issue purchase $installationId "Nombre del titular" $privateKey ".\licencia.json"
node scripts/license-issuer.mjs issue rental $installationId "Nombre del titular" $privateKey ".\licencia-alquiler.json" "2026-12-31"
```

Importa el JSON generado en la aplicación. El emisor local crea la clave privada en
`%APPDATA%\CashFlowPOS\license-issuer\license-private.pem`; respáldala de forma segura
y no la incluyas en el repositorio ni la compartas con clientes. La app contiene solo
la clave pública. Si se pierde o reemplaza la clave privada, las instalaciones existentes
no podrán validar licencias emitidas con la nueva clave sin actualizar la clave pública
y distribuir una nueva versión.

La activación y la comprobación de vencimiento son offline. La entrega del código fuente
se acuerda por separado y no la realiza el archivo de activación. Actualmente el bloqueo
de licencia se aplica en la interfaz de la aplicación; no es DRM frente a quien modifique
o recompile el programa.

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
