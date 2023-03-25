# **Calculated Finance Integration Tests**

## Running the tests

If you use vscode, you can run the tests by pressing `F5` or by clicking the `Run` button in the debug tab. Ensure that you have a launch.config in the .vscode folder at the root of the project. It should look something like

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "node",
      "request": "launch",
      "name": "Run Tests",
      "program": "${workspaceFolder}/tests/node_modules/mocha/bin/_mocha",
      "args": ["-r", "dotenv/config", "--recursive", "**/*.test.ts", "--timeout", "300000", "--exit"],
      "console": "integratedTerminal",
      "internalConsoleOptions": "neverOpen",
      "cwd": "${workspaceFolder}/tests"
    }
  ]
}
```
