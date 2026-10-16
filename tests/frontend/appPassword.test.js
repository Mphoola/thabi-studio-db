import { describe, expect, it } from "vitest";
import { isStrongPassword, passwordRules } from "../../src/lib/appPassword.js";

const unmet = (password) =>
  passwordRules(password)
    .filter((rule) => !rule.met)
    .map((rule) => rule.id);

describe("app password rules", () => {
  it("needs ten characters and both cases", () => {
    expect(unmet("")).toEqual(["length", "upper", "lower"]);
    expect(unmet("short Pw")).toEqual(["length"]);
    expect(unmet("alllowercase")).toEqual(["upper"]);
    expect(unmet("ALLUPPERCASE")).toEqual(["lower"]);
    expect(isStrongPassword("Long enough Pw")).toBe(true);
  });

  it("counts characters, not UTF-16 units", () => {
    expect(unmet("Ab😀😀😀😀😀😀😀")).toEqual(["length"]);
  });
});
