import { execFileSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import openapiTS, { astToString } from "openapi-typescript";
import prettier from "prettier";
import ts from "typescript";

const root = new URL("../", import.meta.url);
const schema = JSON.parse(
  execFileSync(
    "cargo",
    [
      "run",
      "--quiet",
      "-p",
      "stockfront-adapters",
      "--example",
      "export_openapi",
    ],
    { cwd: new URL("../", root), encoding: "utf8" },
  ),
);
const ast = await openapiTS(schema, {
  rootTypes: true,
  rootTypesNoSchemaPrefix: true,
  rootTypesKeepCasing: true,
  transform(schema) {
    if (schema.format === "evm-address" || schema.format === "hex") {
      return ts.factory.createTemplateLiteralType(
        ts.factory.createTemplateHead("0x"),
        [
          ts.factory.createTemplateLiteralTypeSpan(
            ts.factory.createKeywordTypeNode(ts.SyntaxKind.StringKeyword),
            ts.factory.createTemplateTail(""),
          ),
        ],
      );
    }
  },
});

for (const [path, content, parser] of [
  ["openapi.json", JSON.stringify(schema), "json"],
  [
    "src/api/generated.ts",
    "// Generated from Rust. Run pnpm codegen; do not edit.\n" +
      astToString(ast),
    "typescript",
  ],
]) {
  const output = await prettier.format(content, { parser });
  const file = new URL(path, root);
  if (process.argv.includes("--check")) {
    const current = await readFile(file, "utf8").catch(() => "");
    if (current !== output) {
      throw new Error(`${path} is stale. Run pnpm codegen.`);
    }
  } else {
    await writeFile(file, output);
  }
}
