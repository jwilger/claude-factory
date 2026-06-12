# Strict Linting — TypeScript

## ESLint with TypeScript strict config

Use `typescript-eslint` v8+ with the `strictTypeChecked` preset as the baseline. Enable additional rules beyond the preset for factory projects.

```js
// eslint.config.mjs
import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  eslint.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  {
    rules: {
      // No any — ever. Cast through unknown if a third-party boundary forces it.
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-unsafe-assignment': 'error',
      '@typescript-eslint/no-unsafe-call': 'error',
      '@typescript-eslint/no-unsafe-member-access': 'error',
      '@typescript-eslint/no-unsafe-return': 'error',
      '@typescript-eslint/no-unsafe-argument': 'error',

      // Exhaustiveness — every switch/if-else on a union must handle all cases.
      '@typescript-eslint/switch-exhaustiveness-check': 'error',

      // Nullability discipline.
      '@typescript-eslint/no-non-null-assertion': 'error',
      '@typescript-eslint/strict-null-checks': 'error', // enforced in tsconfig, surfaced here

      // Return types must be explicit on exported functions.
      '@typescript-eslint/explicit-module-boundary-types': 'error',

      // No floating promises — every async operation must be awaited or explicitly void'd.
      '@typescript-eslint/no-floating-promises': 'error',
      '@typescript-eslint/no-misused-promises': 'error',

      // Prefer readonly — immutability by default.
      '@typescript-eslint/prefer-readonly': 'error',
      '@typescript-eslint/prefer-readonly-parameter-types': 'warn', // upgrade to error once team adapts

      // No unused variables (typescript-eslint version handles type-only imports correctly).
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],

      // Consistency.
      '@typescript-eslint/consistent-type-imports': 'error',
      '@typescript-eslint/consistent-type-exports': 'error',
    },
  }
);
```

## tsconfig.json — strictness baseline

```json
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "exactOptionalPropertyTypes": true,
    "noPropertyAccessFromIndexSignature": true,
    "forceConsistentCasingInFileNames": true,
    "useUnknownInCatchVariables": true
  }
}
```

`noUncheckedIndexedAccess` is the single highest-value addition beyond `strict: true` — it makes `array[i]` return `T | undefined` instead of `T`, catching the most common runtime crash pattern.

## Biome (alternative to ESLint)

If using Biome instead of ESLint:

```json
{
  "$schema": "https://biomejs.dev/schemas/1.9.0/schema.json",
  "linter": {
    "enabled": true,
    "rules": {
      "recommended": true,
      "suspicious": { "recommended": true, "noExplicitAny": "error" },
      "correctness": { "recommended": true, "useExhaustiveDependencies": "error" },
      "style": { "recommended": true, "noNonNullAssertion": "error" },
      "complexity": { "recommended": true }
    }
  },
  "formatter": {
    "enabled": true,
    "indentStyle": "space",
    "indentWidth": 2
  }
}
```

## When to relax

A relaxation requires: a `// eslint-disable-next-line` comment with an explicit `// Reason:` comment on the following line explaining why the rule cannot apply here. Generic `any` casts at JSON/external-API parse boundaries are the canonical justified use — always immediately wrapped in a branded type or schema validation.
