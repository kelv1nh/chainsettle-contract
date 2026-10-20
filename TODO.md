# TODO

- [x] Create / update docs/SECURITY.md with security model, trust assumptions, threat vectors w/ mitigations, known limitations, and disclosure contact.
- [x] Link docs/SECURITY.md from README (Security Considerations section).
- [x] Ensure README link matches acceptance criteria.
- [ ] Run repo checks (cargo test) if desired.

const request = require("supertest");
const app = require("../../app");

jest.mock("../../services/assetHolders", () => ({
  getAssetHolders: jest.fn(),
}));

jest.mock("../../services/dexPrice", () => ({
  getCurrentPriceXLM: jest.fn(),
}));

const {
  getAssetHolders,
} = require("../../services/assetHolders");

const {
  getCurrentPriceXLM,
} = require("../../services/dexPrice");

describe("GET /asset/:code/:issuer/market-cap", () => {
  const code = "TEST";
  const issuer = "GISSUER123";

  const url = () => `/asset/${code}/${issuer}/market-cap`;

  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe("circulating supply", () => {
    test("excludes the issuer's 10% holding", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "10.0000000" },
        { account: "GHOLDER1", balance: "40.0000000" },
        { account: "GHOLDER2", balance: "50.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(2);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("90.0000000");
    });

    test("returns zero when only the issuer holds the asset", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "1000.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(3);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("0.0000000");
      expect(response.body.marketCapXLM).toBe(0);
    });

    test("returns zero when the holder list is empty", async () => {
      getAssetHolders.mockResolvedValue([]);
      getCurrentPriceXLM.mockResolvedValue(1);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("0.0000000");
    });

    test("does not subtract an issuer balance when it is absent", async () => {
      getAssetHolders.mockResolvedValue([
        { account: "GHOLDER1", balance: "25.0000000" },
        { account: "GHOLDER2", balance: "75.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(1);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("100.0000000");
    });

    test("does not include the issuer in circulating supply", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "500.0000000" },
        { account: "GHOLDER1", balance: "200.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(0.1);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("200.0000000");
    });
  });

  describe("market capitalization", () => {
    test("computes market cap as circulating supply times price", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "100.0000000" },
        { account: "GHOLDER1", balance: "600.0000000" },
        { account: "GHOLDER2", balance: "300.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(0.5);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("900.0000000");
      expect(response.body.currentPriceXLM).toBe(0.5);
      expect(response.body.marketCapXLM).toBe(450);
    });

    test("returns zero market cap for zero circulating supply", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "100.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(10);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.marketCapXLM).toBe(0);
    });

    test("returns null market cap when price is unavailable", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "10.0000000" },
        { account: "GHOLDER1", balance: "90.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(null);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.currentPriceXLM).toBeNull();
      expect(response.body.marketCapXLM).toBeNull();
      expect(response.body.circulatingSupply).toBe("90.0000000");
    });

    test("supports a zero market price", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "10.0000000" },
        { account: "GHOLDER1", balance: "90.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(0);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.currentPriceXLM).toBe(0);
      expect(response.body.marketCapXLM).toBe(0);
    });

    test("preserves a positive circulating supply at a small price", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "1.0000000" },
        { account: "GHOLDER1", balance: "999.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(0.000001);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("999.0000000");
      expect(response.body.currentPriceXLM).toBe(0.000001);
      expect(response.body.marketCapXLM).toBeCloseTo(0.000999);
    });
  });

  describe("mock verification", () => {
    test("requests holder data for the requested asset", async () => {
      getAssetHolders.mockResolvedValue([]);
      getCurrentPriceXLM.mockResolvedValue(null);

      await request(app).get(url());

      expect(getAssetHolders).toHaveBeenCalledWith(code, issuer);
    });

    test("requests DEX price data for the requested asset", async () => {
      getAssetHolders.mockResolvedValue([]);
      getCurrentPriceXLM.mockResolvedValue(null);

      await request(app).get(url());

      expect(getCurrentPriceXLM).toHaveBeenCalledWith(code, issuer);
    });

    test("queries both data sources once", async () => {
      getAssetHolders.mockResolvedValue([]);
      getCurrentPriceXLM.mockResolvedValue(null);

      await request(app).get(url());

      expect(getAssetHolders).toHaveBeenCalledTimes(1);
      expect(getCurrentPriceXLM).toHaveBeenCalledTimes(1);
    });
  });
});


const request = require("supertest");
const app = require("../../app");

jest.mock("../../services/assetHolders", () => ({
  getAssetHolders: jest.fn(),
}));

jest.mock("../../services/dexPrice", () => ({
  getCurrentPriceXLM: jest.fn(),
}));

const {
  getAssetHolders,
} = require("../../services/assetHolders");

const {
  getCurrentPriceXLM,
} = require("../../services/dexPrice");

describe("GET /asset/:code/:issuer/market-cap", () => {
  const code = "TEST";
  const issuer = "GISSUER123";

  const url = () => `/asset/${code}/${issuer}/market-cap`;

  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe("circulating supply", () => {
    test("excludes the issuer's 10% holding", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "10.0000000" },
        { account: "GHOLDER1", balance: "40.0000000" },
        { account: "GHOLDER2", balance: "50.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(2);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("90.0000000");
    });

    test("returns zero when only the issuer holds the asset", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "1000.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(3);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("0.0000000");
      expect(response.body.marketCapXLM).toBe(0);
    });

    test("returns zero when the holder list is empty", async () => {
      getAssetHolders.mockResolvedValue([]);
      getCurrentPriceXLM.mockResolvedValue(1);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("0.0000000");
    });

    test("does not subtract an issuer balance when it is absent", async () => {
      getAssetHolders.mockResolvedValue([
        { account: "GHOLDER1", balance: "25.0000000" },
        { account: "GHOLDER2", balance: "75.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(1);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("100.0000000");
    });

    test("does not include the issuer in circulating supply", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "500.0000000" },
        { account: "GHOLDER1", balance: "200.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(0.1);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("200.0000000");
    });
  });

  describe("market capitalization", () => {
    test("computes market cap as circulating supply times price", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "100.0000000" },
        { account: "GHOLDER1", balance: "600.0000000" },
        { account: "GHOLDER2", balance: "300.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(0.5);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("900.0000000");
      expect(response.body.currentPriceXLM).toBe(0.5);
      expect(response.body.marketCapXLM).toBe(450);
    });

    test("returns zero market cap for zero circulating supply", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "100.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(10);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.marketCapXLM).toBe(0);
    });

    test("returns null market cap when price is unavailable", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "10.0000000" },
        { account: "GHOLDER1", balance: "90.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(null);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.currentPriceXLM).toBeNull();
      expect(response.body.marketCapXLM).toBeNull();
      expect(response.body.circulatingSupply).toBe("90.0000000");
    });

    test("supports a zero market price", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "10.0000000" },
        { account: "GHOLDER1", balance: "90.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(0);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.currentPriceXLM).toBe(0);
      expect(response.body.marketCapXLM).toBe(0);
    });

    test("preserves a positive circulating supply at a small price", async () => {
      getAssetHolders.mockResolvedValue([
        { account: issuer, balance: "1.0000000" },
        { account: "GHOLDER1", balance: "999.0000000" },
      ]);

      getCurrentPriceXLM.mockResolvedValue(0.000001);

      const response = await request(app).get(url());

      expect(response.status).toBe(200);
      expect(response.body.circulatingSupply).toBe("999.0000000");
      expect(response.body.currentPriceXLM).toBe(0.000001);
      expect(response.body.marketCapXLM).toBeCloseTo(0.000999);
    });
  });

  describe("mock verification", () => {
    test("requests holder data for the requested asset", async () => {
      getAssetHolders.mockResolvedValue([]);
      getCurrentPriceXLM.mockResolvedValue(null);

      await request(app).get(url());

      expect(getAssetHolders).toHaveBeenCalledWith(code, issuer);
    });

    test("requests DEX price data for the requested asset", async () => {
      getAssetHolders.mockResolvedValue([]);
      getCurrentPriceXLM.mockResolvedValue(null);

      await request(app).get(url());

      expect(getCurrentPriceXLM).toHaveBeenCalledWith(code, issuer);
    });

    test("queries both data sources once", async () => {
      getAssetHolders.mockResolvedValue([]);
      getCurrentPriceXLM.mockResolvedValue(null);

      await request(app).get(url());

      expect(getAssetHolders).toHaveBeenCalledTimes(1);
      expect(getCurrentPriceXLM).toHaveBeenCalledTimes(1);
    });
  });
});

