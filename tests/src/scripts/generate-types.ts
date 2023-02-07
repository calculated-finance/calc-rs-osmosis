import { compile } from 'json-schema-to-typescript';
import { writeFileSync, readFileSync, mkdirSync } from 'fs';

console.log('\x1b[36m%s\x1b[0m', `generating...`);

const pathToSchema = process.argv[2];

const pathToOutput = process.argv[3] ?? './types';

console.log('\x1b[35m%s\x1b[0m', `path to schema: ${pathToSchema}\npath to output: ${pathToOutput}`);

const rawSchema = readFileSync(pathToSchema, 'utf-8');

const schema = JSON.parse(rawSchema);

const types = ['instantiate', 'migrate', 'execute', 'query'];

mkdirSync(`${pathToOutput}/response`, { recursive: true });

types
  .filter((t) => schema[t])
  .forEach(async (t) => {
    try {
      console.log('\x1b[33m', `compiling client for ${t}`);
      const client = await compile(schema[t], t);
      writeFileSync(`${pathToOutput}/${t}.d.ts`, client);
    } catch (e) {
      console.log(t, e);
    }
  });

Object.keys(schema.responses).forEach(async (k) => {
  try {
    console.log('\x1b[34m', `compiling response types for ${k}`);

    const responseTypes = await compile(schema.responses[k], k);
    writeFileSync(`${pathToOutput}/response/${k}.d.ts`, responseTypes);
  } catch (e) {
    console.log(k, e);
  }
});

console.log('\x1b[36m%s\x1b[0m', `finished successfully`);
