# TypeScript Code Generation from OpenAPI

This directory contains TypeScript types auto-generated from the OpenAPI specification.

## Overview

The project uses **openapi-typescript** to automatically generate TypeScript types from the OpenAPI specification (`docs/openapi.yaml`). This ensures type safety and consistency between the API contract and the frontend implementation.

## Generated Files

- **`api.d.ts`** - Auto-generated TypeScript types from OpenAPI specification

## Usage

### Importing Generated Types

```typescript
import type { components, operations } from '../types/api';

// Use schema types
type Pipeline = components['schemas']['Pipeline'];
type CreatePipelineRequest = components['schemas']['CreatePipelineRequest'];

// Use operation types
type ListPipelinesResponse = operations['listPipelines']['responses'][200]['content']['application/json'];
```

### Example: Using with API Services

```typescript
import type { components } from '../types/api';
import { pipelineApi } from './pipelineApi';

type Pipeline = components['schemas']['Pipeline'];

async function fetchPipelines() {
  const pipelines: Pipeline[] = await pipelineApi.listPipelines();
  console.log(pipelines);
}
```

## Generation Process

### Manual Generation

```bash
cd web-console
npm run generate:types
```

### Watch Mode (Auto-regenerate on changes)

```bash
cd web-console
npm run generate:types:watch
```

### Using Makefile

```bash
make gen-types
```

### CI/CD Integration

The `scripts/validate-api-contract.sh` script automatically generates types during the contract validation process.

## Configuration

Generation is configured via:

1. **`openapi-typescript.config.json`** - openapi-typescript configuration
2. **`package.json`** - npm scripts for generation
3. **`Makefile`** - make targets for type generation

## Type Safety Benefits

1. **Compile-time validation** - TypeScript catches mismatches between API and frontend
2. **Better IDE support** - Autocomplete and IntelliSense for API responses
3. **Self-documenting code** - Types serve as live documentation
4. **Refactoring safety** - Changes to API contract are immediately reflected in types

## Testing

Generated types are validated through:

1. **Unit tests** - `src/services/__tests__/generatedTypes.test.ts`
   - Validates type structure
   - Tests optional/required fields
   - Verifies enum values

2. **Integration tests** - `src/services/__tests__/pipelineApi.test.ts`
   - Tests API services using generated types
   - Validates request/response matching OpenAPI schema

Run tests:

```bash
cd web-console
npm test -- src/services/__tests__/generatedTypes.test.ts --run
npm test -- src/services/__tests__/pipelineApi.test.ts --run
```

## Best Practices

### 1. Regenerate Types When API Changes

Always regenerate types after modifying the OpenAPI specification:

```bash
npm run generate:types
```

### 2. Use Type Aliases for Better Readability

```typescript
// Instead of using full paths everywhere
type Pipeline = components['schemas']['Pipeline'];

// Or create aliases in your service files
import type { components } from '../types/api';
type Pipeline = components['schemas']['Pipeline'];
```

### 3. Type Guards for Runtime Validation

```typescript
const validStatuses = ['active', 'inactive', 'archived'] as const;

function isValidStatus(status: string): status is typeof validStatuses[number] {
  return validStatuses.includes(status as typeof validStatuses[number]);
}
```

### 4. Handle Optional Fields

```typescript
const pipeline: Pipeline = {
  id: '123',
  name: 'My Pipeline',
  status: 'active',
  // description, schedule, tags are optional
};

if (pipeline.description) {
  console.log(pipeline.description);
}
```

## OpenAPI Specification

The OpenAPI specification is located at `docs/openapi.yaml`. It defines:

- **Paths** - API endpoints (e.g., `/pipelines`, `/worker-pools`)
- **Schemas** - Data models (e.g., `Pipeline`, `CreatePipelineRequest`)
- **Responses** - Response structures and status codes
- **Parameters** - Query, path, and header parameters

## Troubleshooting

### Types Not Updating

1. Ensure the OpenAPI spec is valid: `swagger-cli validate docs/openapi.yaml`
2. Regenerate types: `npm run generate:types`
3. Clear TypeScript cache: Delete `node_modules/.cache` or run `tsc --build --clean`

### Type Errors

1. Check that the OpenAPI spec has proper schema definitions
2. Ensure all referenced schemas exist in the `components/schemas` section
3. Validate enum values match between spec and usage

### Missing Types

1. Verify the schema is properly referenced in paths
2. Check that required fields are marked correctly in the spec
3. Ensure `components/schemas` section includes all necessary models

## References

- [openapi-typescript Documentation](https://github.com/openapits/openapi-typescript)
- [OpenAPI Specification](https://swagger.io/specification/)
- [TypeScript Handbook](https://www.typescriptlang.org/docs/)
