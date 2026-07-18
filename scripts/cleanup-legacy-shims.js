#!/usr/bin/env node

// fnva postinstall self-heal hook.
//
// Legacy fnva (<0.0.87) copied bin/fnva.ps1 into the Node executable directory
// (its guess for the npm global bin dir). Under version managers like fnm/nvm,
// process.execPath points at a per-shell shim dir that sits EARLIER on PATH
// than the real npm global dir, so that stray fnva.ps1 shadowed npm's genuine
// launcher and `fnva` failed with "native binary not found" — including right
// after `npm i -g fnva@latest`, because an upgrade never triggered the
// postuninstall cleanup that removed the stray.
//
// This hook closes that gap: it runs on every install/upgrade and silently
// removes the stray shim. It deliberately does NOT touch the user's shell
// profile (that is the postuninstall hook's job) and reuses the existing,
// npm-prefix-guarded cleanupLegacyPs1() so npm's own legitimate fnva.ps1 is
// never removed.

const { cleanupLegacyPs1 } = require('./uninstall-shell-integration.js');

cleanupLegacyPs1();
