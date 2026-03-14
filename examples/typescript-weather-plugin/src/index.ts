/**
 * TypeScript Weather Plugin for OpenClaw+
 * 
 * Demonstrates multi-language WASM plugin support.
 * Compiled to WASM via Javy (QuickJS runtime).
 * 
 * Exposes:
 *   - weather.forecast(city: string) → 7-day forecast
 *   - weather.current(city: string) → current conditions
 */

interface SkillManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  skills: Array<{ name: string; description: string; parameters: any }>;
}

interface ExecuteRequest {
  skill: string;
  args: Record<string, any>;
  request_id: string;
}

interface ExecuteResponse {
  ok: boolean;
  request_id: string;
  observation?: string;
  error?: string;
}

// ── Manifest ──────────────────────────────────────────────────────────────

function skill_manifest(): string {
  const manifest: SkillManifest = {
    id: "com.example.typescript-weather",
    name: "TypeScript Weather Plugin",
    version: "1.0.0",
    description: "Weather forecast and current conditions via TypeScript/WASM",
    skills: [
      {
        name: "weather.forecast",
        description: "Get 7-day weather forecast for a city",
        parameters: {
          type: "object",
          properties: {
            city: { type: "string", description: "City name (e.g. 'London', 'Tokyo')" }
          },
          required: ["city"]
        }
      },
      {
        name: "weather.current",
        description: "Get current weather conditions for a city",
        parameters: {
          type: "object",
          properties: {
            city: { type: "string", description: "City name" }
          },
          required: ["city"]
        }
      }
    ]
  };
  return JSON.stringify(manifest);
}

// ── Skill implementations ─────────────────────────────────────────────────

function weatherForecast(city: string): string {
  // Mock 7-day forecast (in production, call external API via host_http_fetch)
  const days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const conditions = ["Sunny", "Cloudy", "Rainy", "Partly Cloudy"];
  
  let forecast = `7-Day Forecast for ${city}:\n\n`;
  for (let i = 0; i < 7; i++) {
    const temp = Math.floor(Math.random() * 15) + 15; // 15-30°C
    const condition = conditions[Math.floor(Math.random() * conditions.length)];
    forecast += `${days[i]}: ${condition}, ${temp}°C\n`;
  }
  
  return forecast;
}

function weatherCurrent(city: string): string {
  // Mock current conditions
  const temp = Math.floor(Math.random() * 20) + 10; // 10-30°C
  const humidity = Math.floor(Math.random() * 40) + 40; // 40-80%
  const conditions = ["Clear", "Cloudy", "Light Rain", "Partly Cloudy"];
  const condition = conditions[Math.floor(Math.random() * conditions.length)];
  
  return `Current weather in ${city}:
  Condition: ${condition}
  Temperature: ${temp}°C
  Humidity: ${humidity}%
  Wind: ${Math.floor(Math.random() * 20)} km/h`;
}

// ── Execute dispatcher ────────────────────────────────────────────────────

function skill_execute(requestJson: string): string {
  try {
    const req: ExecuteRequest = JSON.parse(requestJson);
    
    let observation: string;
    
    switch (req.skill) {
      case "weather.forecast":
        if (!req.args.city) {
          throw new Error("Missing required parameter: city");
        }
        observation = weatherForecast(req.args.city as string);
        break;
        
      case "weather.current":
        if (!req.args.city) {
          throw new Error("Missing required parameter: city");
        }
        observation = weatherCurrent(req.args.city as string);
        break;
        
      default:
        throw new Error(`Unknown skill: ${req.skill}`);
    }
    
    const response: ExecuteResponse = {
      ok: true,
      request_id: req.request_id,
      observation
    };
    
    return JSON.stringify(response);
    
  } catch (error) {
    const response: ExecuteResponse = {
      ok: false,
      request_id: "unknown",
      error: error instanceof Error ? error.message : String(error)
    };
    return JSON.stringify(response);
  }
}

// ── WASM exports (Javy runtime will bind these) ───────────────────────────

// @ts-ignore - Javy provides these globals
globalThis.skill_manifest = skill_manifest;
// @ts-ignore
globalThis.skill_execute = skill_execute;
