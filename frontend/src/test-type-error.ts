// Deliberate TypeScript error for testing CI failure
// This file should cause the typecheck step to fail in CI

interface TestInterface {
  name: string;
  age: number;
}

const testObject: TestInterface = {
  name: "test",
  age: "not a number" // This should cause a type error
};

export default testObject;