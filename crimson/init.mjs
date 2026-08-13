import { execSync } from 'child_process';
execSync('npx tauri init --ci --app-name Crimsons --window-title "Crimsons" --frontend-dist ../dist --dev-url http://localhost:5173 --before-dev-command "npm run dev" --before-build-command "npm run build"', { stdio: 'inherit' });
