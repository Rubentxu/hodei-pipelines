# Hodei API Postman & Automation Guide

This guide details how to use the enhanced Postman collection for manual testing and CI/CD automation.

## 1. Collection Overview

The `hodei_api_postman_collection.json` is organized into three main sections:

1.  **Worker Pools**: CRUD operations for managing resource pools.
2.  **E2E Pipeline Scenario**: A sequential workflow (Create -> Execute -> Verify -> Delete) designed to test the full lifecycle of a pipeline.
3.  **Negative Tests**: Requests designed to fail (e.g., 404 Not Found, 400 Bad Request) to verify error handling.

## 2. Quick Start (Manual)

1.  **Import**: Drag `docs/testing/hodei_api_postman_collection.json` into Postman.
2.  **Configure**: Set the `baseUrl` collection variable (default: `http://localhost:8080`).
3.  **Run**:
    *   Open the collection folder.
    *   Click **Run**.
    *   Select "Run manually" and click **Run Hodei API (Enhanced)**.

## 3. Automation with Newman (CLI)

[Newman](https://www.npmjs.com/package/newman) is the command-line collection runner for Postman. It allows you to run these tests in a CI/CD pipeline.

### Prerequisites
*   Node.js (>= v10)
*   npm

### Installation
```bash
npm install -g newman
```

### Running Tests
To run the entire collection from the command line:

```bash
newman run docs/testing/hodei_api_postman_collection.json \
  --env-var "baseUrl=http://localhost:8080" \
  --reporters cli,json \
  --reporter-json-export docs/testing/report.json
```

**Options Explained:**
*   `--env-var "baseUrl=..."`: Overrides the base URL for the target environment.
*   `--reporters cli,json`: Outputs results to the console and saves a JSON report.
*   `--reporter-json-export`: Specifies where to save the JSON report.

### CI/CD Integration Example (GitHub Actions)

Add this step to your `.github/workflows/test.yml`:

```yaml
- name: Run API Tests
  run: |
    npm install -g newman
    newman run docs/testing/hodei_api_postman_collection.json --env-var "baseUrl=http://localhost:8080"
```

## 4. Test Details

### Schema Validation
We use `tv4` (Tiny Validator 4) within Postman scripts to validate that JSON responses match the expected schema.
*   *Example:* Verifying that a "Create Pool" response contains an `id` string and a `config` object.

### Dynamic Variables
The collection is stateful. It captures IDs from responses and saves them to **Collection Variables**.
*   `poolId`: ID of the pool created in the "Worker Pools" folder.
*   `e2ePipelineId`: ID of the pipeline created in the "E2E Scenario".
*   `e2eExecutionId`: ID of the execution started in the "E2E Scenario".

**Note:** If you run requests out of order manually, these variables might be missing. It is recommended to run the folders sequentially.
