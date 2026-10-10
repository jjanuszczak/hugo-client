import { cp, mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

const projectRoot = fileURLToPath(new URL("../", import.meta.url));
const output = join(projectRoot, "dist");
await rm(output, { recursive: true, force: true });
await mkdir(output, { recursive: true });
await cp(join(projectRoot, "frontend"), output, { recursive: true });
process.stdout.write(`Built browser preview into ${output}\n`);
