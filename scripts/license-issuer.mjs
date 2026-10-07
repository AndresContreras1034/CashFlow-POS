import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import { createPrivateKey, generateKeyPairSync, randomUUID, sign } from 'node:crypto';
import { homedir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const publicKeyInApp = path.join(
  repositoryRoot,
  'src-tauri',
  'src',
  'modules',
  'licensing',
  'public_key.pem',
);
const defaultKeyDirectory = process.env.APPDATA
  ? path.join(process.env.APPDATA, 'CashFlowPOS', 'license-issuer')
  : path.join(homedir(), '.cashflow-pos', 'license-issuer');

function usage() {
  console.log(`Herramienta local de licencias offline

Inicializar llaves:
  node scripts/license-issuer.mjs init [directorio-privado] [--force]

Emitir licencia:
  node scripts/license-issuer.mjs issue <purchase|rental> <installation-id> <titular> <private-key.pem> <salida.json> [vence-YYYY-MM-DD]

La clave privada se conserva localmente y no se copia al repositorio.
El comando init copia únicamente la clave pública a la app; recompílala después.`);
}

async function initializeKeyPair(args) {
  const force = args.includes('--force');
  const keyDirectory = path.resolve(args.find(arg => arg !== '--force') ?? defaultKeyDirectory);
  const privateKeyPath = path.join(keyDirectory, 'license-private.pem');
  const keyPair = generateKeyPairSync('rsa', {
    modulusLength: 2048,
    publicKeyEncoding: { type: 'spki', format: 'pem' },
    privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
  });

  await mkdir(keyDirectory, { recursive: true });
  if (!force) {
    try {
      await readFile(privateKeyPath);
      throw new Error(`Ya existe una clave privada en ${privateKeyPath}; usa --force solo si deseas reemplazarla.`);
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
  }

  await writeFile(privateKeyPath, keyPair.privateKey, { mode: 0o600 });
  await writeFile(publicKeyInApp, keyPair.publicKey, 'utf8');
  console.log(`Clave privada creada en: ${privateKeyPath}`);
  console.log(`Clave pública instalada en: ${publicKeyInApp}`);
  console.log('Respalda la clave privada de forma segura y recompila la aplicación.');
}

async function issueLicense(args) {
  const [kind, installationId, licensee, privateKeyPath, outputPath, expirationDate] = args;
  if (!['purchase', 'rental'].includes(kind) || !installationId || !licensee || !privateKeyPath || !outputPath) {
    usage();
    process.exitCode = 2;
    return;
  }
  if (kind === 'rental' && !expirationDate) {
    throw new Error('Para alquiler debes especificar vence-YYYY-MM-DD.');
  }
  if (kind === 'purchase' && expirationDate) {
    throw new Error('Una compra perpetua no debe llevar fecha de vencimiento.');
  }

  let expiresAt = null;
  if (expirationDate) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(expirationDate)) {
      throw new Error('La fecha debe tener formato YYYY-MM-DD.');
    }
    const endOfDay = new Date(`${expirationDate}T23:59:59.999Z`);
    if (Number.isNaN(endOfDay.valueOf()) || endOfDay.toISOString().slice(0, 10) !== expirationDate) {
      throw new Error('La fecha de vencimiento no es válida.');
    }
    expiresAt = endOfDay.toISOString();
  }

  const payload = {
    schema_version: 1,
    license_id: randomUUID(),
    installation_id: installationId,
    licensee: licensee.trim(),
    kind,
    issued_at: new Date().toISOString(),
    expires_at: expiresAt,
    source_code_access: true,
  };
  if (!payload.licensee) throw new Error('El titular de la licencia es obligatorio.');

  const privateKey = createPrivateKey(await readFile(path.resolve(privateKeyPath), 'utf8'));
  const payloadJson = JSON.stringify(payload);
  const signature = sign('RSA-SHA256', Buffer.from(payloadJson, 'utf8'), privateKey).toString('hex');
  await writeFile(
    path.resolve(outputPath),
    `${JSON.stringify({ payload: payloadJson, signature }, null, 2)}\n`,
    'utf8',
  );
  console.log(`Licencia ${kind === 'purchase' ? 'perpetua' : `de alquiler hasta ${expirationDate}`} emitida: ${path.resolve(outputPath)}`);
  console.log('La licencia firmada incluye acceso al código fuente según el acuerdo de entrega.');
}

const [command, ...args] = process.argv.slice(2);
try {
  if (command === 'init') {
    await initializeKeyPair(args);
  } else if (command === 'issue') {
    await issueLicense(args);
  } else {
    usage();
    process.exitCode = command ? 2 : 0;
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
