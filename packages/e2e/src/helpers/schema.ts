import path from 'node:path';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import jsYaml from 'js-yaml';
import { Ajv2020 } from 'ajv/dist/2020.js';


const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SCHEMA_DIR = path.resolve(__dirname, '../../../../packages/schema');

const ajv = new Ajv2020({ allErrors: true });

const skillSchema = JSON.parse(fs.readFileSync(path.join(SCHEMA_DIR, 'skill.schema.json'), 'utf8'));
const agentSchema = JSON.parse(fs.readFileSync(path.join(SCHEMA_DIR, 'agent.schema.json'), 'utf8'));
const mcpSchema = JSON.parse(fs.readFileSync(path.join(SCHEMA_DIR, 'mcp-server.schema.json'), 'utf8'));

const validateSkillFn = ajv.compile(skillSchema);
const validateAgentFn = ajv.compile(agentSchema);
const validateMcpFn = ajv.compile(mcpSchema);

export interface ValidationResult {
  valid: boolean;
  errors?: string;
}

export function validateFile(filePath: string, schemaType: 'skill' | 'agent' | 'mcp-server'): ValidationResult {
  if (!fs.existsSync(filePath)) {
    return { valid: false, errors: `File does not exist: ${filePath}` };
  }

  const content = fs.readFileSync(filePath, 'utf8');
  let data: any;

  if (filePath.endsWith('.json')) {
    try {
      data = JSON.parse(content);
    } catch (err: any) {
      return { valid: false, errors: `Invalid JSON: ${err.message}` };
    }
  } else if (filePath.endsWith('.yaml') || filePath.endsWith('.yml')) {
    try {
      data = jsYaml.load(content);
    } catch (err: any) {
      return { valid: false, errors: `Invalid YAML: ${err.message}` };
    }
  } else {
    return { valid: false, errors: `Unsupported file extension for schema validation: ${filePath}` };
  }

  return validateData(data, schemaType);
}

export function validateData(data: any, schemaType: 'skill' | 'agent' | 'mcp-server'): ValidationResult {
  let validate;
  switch (schemaType) {
    case 'skill':
      validate = validateSkillFn;
      break;
    case 'agent':
      validate = validateAgentFn;
      break;
    case 'mcp-server':
      validate = validateMcpFn;
      break;
    default:
      return { valid: false, errors: `Unknown schema type: ${schemaType}` };
  }

  const valid = validate(data);
  if (!valid) {
    const errorsStr = ajv.errorsText(validate.errors);
    return { valid: false, errors: errorsStr };
  }

  return { valid: true };
}
