import "@testing-library/jest-dom/vitest";

// Use browser storage even when Node exposes its own localStorage global.
vi.stubGlobal("localStorage", jsdom.window.localStorage);
