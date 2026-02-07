import { writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import * as z from "zod";

import { GlobalConfigSchema, LocalConfigSchema } from "./schema.ts";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../..");
const localOutputPath = resolve(root, "iconmatelocal.schema.json");
const globalOutputPath = resolve(root, "iconmateglobal.schema.json");
const schemaDraft = "https://json-schema.org/draft/2020-12/schema";

function writeJson(filePath: string, json: object) {
  writeFileSync(filePath, `${JSON.stringify(json, null, 2)}\n`, "utf8");
}

const localSchema = z.toJSONSchema(LocalConfigSchema, {
  io: "input",
  target: "draft-2020-12"
});

const globalSchema = z.toJSONSchema(GlobalConfigSchema, {
  io: "input",
  target: "draft-2020-12"
});

const localStandaloneSchema = {
  $schema: schemaDraft,
  $id: "https://raw.githubusercontent.com/Blankeos/iconmate/main/iconmatelocal.schema.json",
  ...localSchema
};

const globalStandaloneSchema = {
  $schema: schemaDraft,
  $id: "https://raw.githubusercontent.com/Blankeos/iconmate/main/iconmateglobal.schema.json",
  ...globalSchema
};

writeJson(localOutputPath, localStandaloneSchema);
writeJson(globalOutputPath, globalStandaloneSchema);

console.log(`Generated ${localOutputPath}`);
console.log(`Generated ${globalOutputPath}`);
