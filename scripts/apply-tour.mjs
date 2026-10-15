// Uso (desde la raíz del proyecto):  node scripts\apply-tour.mjs
// Aplica 3 cambios pequeños. Es seguro ejecutarlo varias veces y crea copias .bak.
import fs from 'node:fs'

const patches = [
  {
    file: 'src/routes/AppRouter.tsx',
    marker: "from '../tour'",
    edits: [
      {
        find: "import Reports from '../pages/Reports/Reports';",
        add: "\nimport { TourHelpButton } from '../tour';",
        where: 'after',
      },
      { find: '</Routes>', add: '\n        <TourHelpButton />', where: 'after' },
    ],
  },
  {
    file: 'src/pages/Settings/Settings.tsx',
    marker: "from '../../tour'",
    edits: [
      {
        find: "| 'backup' | 'developer';",
        replace: "| 'backup' | 'developer' | 'tutorial';",
      },
      {
        find: "{ id: 'developer', label: 'Desarrollador' },",
        add: "\n  { id: 'tutorial', label: 'Tutoriales' },",
        where: 'after',
      },
      {
        find: "import './Settings.css';",
        add: "import { TourPanel } from '../../tour';\n",
        where: 'before',
      },
      {
        find: '{section === \'backup\' && <BackupSettingsForm onToast={addToast} />}',
        add: "\n\n        {section === 'tutorial' && <TourPanel />}",
        where: 'after',
      },
      {
        find: "{section !== 'backup' && (",
        replace: "{section !== 'backup' && section !== 'tutorial' && (",
      },
    ],
  },
  {
    file: 'src/components/layout/Sidebar.tsx',
    marker: 'data-tour-active',
    edits: [
      {
        find: 'const handleShortcut = (event: KeyboardEvent) => {',
        add: "\n      if (document.body.hasAttribute('data-tour-active')) return;",
        where: 'after',
      },
    ],
  },
]

let failed = false

for (const patch of patches) {
  if (!fs.existsSync(patch.file)) {
    console.log(`✗ No existe ${patch.file}`);
    failed = true;
    continue;
  }
  let text = fs.readFileSync(patch.file, 'utf8');
  if (text.includes(patch.marker)) {
    console.log(`= ${patch.file} ya estaba aplicado`);
    continue;
  }
  const eol = text.includes('\r\n') ? '\r\n' : '\n';
  const fix = (s) => s.replace(/\r?\n/g, eol);

  let ok = true;
  for (const edit of patch.edits) {
    const idx = text.indexOf(edit.find);
    if (idx === -1) {
      console.log(`✗ ${patch.file}: no encontré «${edit.find}»`);
      ok = false;
      break;
    }
    if (edit.replace !== undefined) {
      text = text.replace(edit.find, fix(edit.replace));
    } else if (edit.where === 'before') {
      text = text.replace(edit.find, fix(edit.add) + edit.find);
    } else {
      text = text.replace(edit.find, edit.find + fix(edit.add));
    }
  }
  if (!ok) {
    failed = true;
    continue;
  }
  fs.copyFileSync(patch.file, patch.file + '.bak');
  fs.writeFileSync(patch.file, text, 'utf8');
  console.log(`✓ ${patch.file} actualizado (copia: ${patch.file}.bak)`);
}

process.exit(failed ? 1 : 0);
