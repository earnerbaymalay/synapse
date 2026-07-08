// Structural check for the JSON Schemas in this package. Deliberately
// dependency-free (no ajv) so `pnpm test` needs no install step; full
// instance validation against real skill.yaml/agent.md/mcp fixtures lands
// in Phase 3 alongside the adapters that produce them.
const fs = require("node:fs");
const path = require("node:path");

const schemaFiles = fs
  .readdirSync(__dirname)
  .filter((name) => name.endsWith(".schema.json"));

if (schemaFiles.length === 0) {
  console.error("FAIL no *.schema.json files found in packages/schema");
  process.exit(1);
}

let failed = false;

for (const file of schemaFiles) {
  const fullPath = path.join(__dirname, file);
  try {
    const schema = JSON.parse(fs.readFileSync(fullPath, "utf8"));
    for (const key of ["$schema", "$id", "title", "type", "properties", "required"]) {
      if (!(key in schema)) {
        throw new Error(`missing required top-level key "${key}"`);
      }
    }
    if (schema.additionalProperties !== false) {
      throw new Error('"additionalProperties" must be false to catch typos in canonical files');
    }
    console.log(`OK   ${file}`);
  } catch (err) {
    failed = true;
    console.error(`FAIL ${file}: ${err.message}`);
  }
}

process.exit(failed ? 1 : 0);
