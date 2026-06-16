const MAX_HEAT_CELLS = 48;
const MAX_GAMMA_BANDS = 16;
const MAX_CASCADE_POINTS = 24;

const VERTEX_SHADER = `#version 300 es
in vec2 a_position;

void main() {
  gl_Position = vec4(a_position, 0.0, 1.0);
}
`;

const FRAGMENT_SHADER = `#version 300 es
precision highp float;

#define MAX_HEAT_CELLS 48
#define MAX_GAMMA_BANDS 16
#define MAX_CASCADE_POINTS 24

uniform vec2 u_resolution;
uniform float u_time;
uniform float u_intensity;
uniform int u_heatCount;
uniform int u_gammaCount;
uniform int u_cascadeCount;
uniform float u_totalStress;
uniform float u_instability;
uniform float u_liquidityField;
uniform float u_gammaField;
uniform float u_liquidationField;
uniform float u_cascadeField;
uniform vec4 u_heatCells[MAX_HEAT_CELLS];
uniform vec4 u_gammaBands[MAX_GAMMA_BANDS];
uniform vec4 u_cascadePoints[MAX_CASCADE_POINTS];

out vec4 fragColor;

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  float a = hash(i);
  float b = hash(i + vec2(1.0, 0.0));
  float c = hash(i + vec2(0.0, 1.0));
  float d = hash(i + vec2(1.0, 1.0));
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y;
}

vec3 heatPalette(float heat, float gamma, float cascade, float voidField) {
  vec3 voidBlue = vec3(0.01, 0.035, 0.075);
  vec3 cold = vec3(0.02, 0.16, 0.28);
  vec3 mid = vec3(0.95, 0.42, 0.12);
  vec3 hot = vec3(1.0, 0.08, 0.16);
  vec3 gammaColor = vec3(0.45, 0.16, 0.95);
  vec3 cascadeColor = vec3(1.0, 0.78, 0.18);

  vec3 color = mix(voidBlue, cold, 0.35 + voidField * 0.35);
  color = mix(color, mid, smoothstep(0.08, 0.45, heat));
  color = mix(color, hot, smoothstep(0.42, 1.12, heat));
  color += gammaColor * gamma * 0.72;
  color += cascadeColor * cascade * 0.70;
  return color;
}

void main() {
  vec2 uv = gl_FragCoord.xy / max(u_resolution.xy, vec2(1.0));
  vec2 fieldUv = vec2(uv.x, 1.0 - uv.y);
  float wave = noise(fieldUv * vec2(4.0, 7.0) + vec2(u_time * 0.035, -u_time * 0.022));
  float fineNoise = noise(fieldUv * vec2(22.0, 18.0) + u_time * 0.018);
  float heat = 0.0;
  float voidField = 0.0;
  float gamma = 0.0;
  float cascade = 0.0;

  for (int i = 0; i < MAX_HEAT_CELLS; i++) {
    if (i >= u_heatCount) {
      break;
    }
    vec4 cell = u_heatCells[i];
    vec2 center = cell.xy;
    float strength = clamp(cell.z, 0.0, 1.0);
    float side = cell.w;
    float aspect = u_resolution.x / max(u_resolution.y, 1.0);
    vec2 delta = vec2((fieldUv.x - center.x) * aspect, fieldUv.y - center.y);
    float radius = mix(0.055, 0.175, strength) * mix(0.75, 1.35, u_intensity);
    float blob = exp(-dot(delta, delta) / max(radius * radius, 0.0001));
    float directionalTint = side > 0.0 ? 1.04 : 0.92;
    heat += blob * strength * directionalTint;
    voidField += (1.0 - smoothstep(0.0, radius * 1.55, length(delta))) * (1.0 - strength) * 0.16;
  }

  for (int i = 0; i < MAX_GAMMA_BANDS; i++) {
    if (i >= u_gammaCount) {
      break;
    }
    vec4 band = u_gammaBands[i];
    float y = band.x;
    float strength = clamp(band.y, 0.0, 1.0);
    float role = band.z;
    float distanceToBand = abs(fieldUv.y - y);
    float wall = exp(-(distanceToBand * distanceToBand) / max(0.00004, pow(0.010 + strength * 0.026, 2.0)));
    float shimmer = 0.72 + 0.28 * sin(u_time * (1.2 + strength) + fieldUv.x * 18.0);
    gamma += wall * strength * shimmer * (role > 0.0 ? 1.08 : 0.88);
  }

  for (int i = 0; i < MAX_CASCADE_POINTS; i++) {
    if (i >= u_cascadeCount) {
      break;
    }
    vec4 point = u_cascadePoints[i];
    vec2 center = point.xy;
    float strength = clamp(point.z, 0.0, 1.0);
    float radius = 0.026 + point.w * 0.090 + sin(u_time * 1.8 + float(i)) * 0.006;
    float d = distance(fieldUv, center);
    float ring = smoothstep(0.014, 0.0, abs(d - radius));
    float vector = smoothstep(0.05, 0.0, abs(fieldUv.x - center.x)) * smoothstep(0.22, 0.0, abs(fieldUv.y - center.y));
    cascade += (ring + vector * 0.18) * strength;
  }

  heat += u_liquidityField * (0.14 + wave * 0.18) + u_liquidationField * 0.18;
  gamma += u_gammaField * (0.16 + fineNoise * 0.08);
  cascade += u_cascadeField * (0.18 + wave * 0.12);

  heat = clamp(heat * (0.64 + wave * 0.22 + fineNoise * 0.10 + u_totalStress * 0.18), 0.0, 1.8);
  gamma = clamp(gamma, 0.0, 1.2);
  cascade = clamp(cascade, 0.0, 1.1);

  vec3 color = heatPalette(heat, gamma, cascade, clamp(voidField + wave * 0.18, 0.0, 1.0));
  color += vec3(1.0, 0.16, 0.20) * u_instability * 0.16;
  float vignette = smoothstep(0.92, 0.18, distance(fieldUv, vec2(0.52, 0.52)));
  color *= 0.62 + vignette * 0.66;
  float alpha = clamp(0.42 + heat * 0.34 + gamma * 0.26 + cascade * 0.32 + u_totalStress * 0.08, 0.34, 0.94);

  fragColor = vec4(color, alpha);
}
`;

export function createForceFieldRenderer(canvas) {
  const gl = canvas?.getContext?.("webgl2", {
    alpha: true,
    antialias: false,
    depth: false,
    powerPreference: "high-performance",
    premultipliedAlpha: true,
  });
  if (!gl) return null;

  const program = createProgram(gl, VERTEX_SHADER, FRAGMENT_SHADER);
  if (!program) return null;

  const positionBuffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
  gl.bufferData(
    gl.ARRAY_BUFFER,
    new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
    gl.STATIC_DRAW,
  );

  const positionLocation = gl.getAttribLocation(program, "a_position");
  const uniforms = {
    resolution: gl.getUniformLocation(program, "u_resolution"),
    time: gl.getUniformLocation(program, "u_time"),
    intensity: gl.getUniformLocation(program, "u_intensity"),
    heatCount: gl.getUniformLocation(program, "u_heatCount"),
    gammaCount: gl.getUniformLocation(program, "u_gammaCount"),
    cascadeCount: gl.getUniformLocation(program, "u_cascadeCount"),
    totalStress: gl.getUniformLocation(program, "u_totalStress"),
    instability: gl.getUniformLocation(program, "u_instability"),
    liquidityField: gl.getUniformLocation(program, "u_liquidityField"),
    gammaField: gl.getUniformLocation(program, "u_gammaField"),
    liquidationField: gl.getUniformLocation(program, "u_liquidationField"),
    cascadeField: gl.getUniformLocation(program, "u_cascadeField"),
    heatCells: gl.getUniformLocation(program, "u_heatCells[0]"),
    gammaBands: gl.getUniformLocation(program, "u_gammaBands[0]"),
    cascadePoints: gl.getUniformLocation(program, "u_cascadePoints[0]"),
  };

  gl.useProgram(program);
  gl.enableVertexAttribArray(positionLocation);
  gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  return {
    render(input = {}) {
      resizeCanvasToDisplaySize(canvas);
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.useProgram(program);
      gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
      gl.enableVertexAttribArray(positionLocation);
      gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);
      gl.uniform2f(uniforms.resolution, canvas.width, canvas.height);
      gl.uniform1f(uniforms.time, Number(input.time || 0));
      gl.uniform1f(uniforms.intensity, clamp01(Number(input.intensity || 0.78)));
      const fieldState = normalizeFieldState(input.fieldState);
      gl.uniform1f(uniforms.totalStress, fieldState.totalStress);
      gl.uniform1f(uniforms.instability, fieldState.instabilityIndex);
      gl.uniform1f(uniforms.liquidityField, fieldState.liquidityField);
      gl.uniform1f(uniforms.gammaField, fieldState.gammaField);
      gl.uniform1f(uniforms.liquidationField, fieldState.liquidationField);
      gl.uniform1f(uniforms.cascadeField, fieldState.cascadeField);

      const heatCells = packHeatCells(input.heatCells);
      const gammaBands = packGammaBands(input.gammaBands);
      const cascadePoints = packCascadePoints(input.cascadePoints);
      gl.uniform1i(uniforms.heatCount, heatCells.count);
      gl.uniform1i(uniforms.gammaCount, gammaBands.count);
      gl.uniform1i(uniforms.cascadeCount, cascadePoints.count);
      gl.uniform4fv(uniforms.heatCells, heatCells.buffer);
      gl.uniform4fv(uniforms.gammaBands, gammaBands.buffer);
      gl.uniform4fv(uniforms.cascadePoints, cascadePoints.buffer);

      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
    },
    dispose() {
      gl.deleteBuffer(positionBuffer);
      gl.deleteProgram(program);
    },
  };
}

function normalizeFieldState(fieldState = {}) {
  return {
    totalStress: clamp01(Number(fieldState.totalStress || 0)),
    instabilityIndex: clamp01(Number(fieldState.instabilityIndex || 0)),
    liquidityField: clamp01(Number(fieldState.liquidityField || 0)),
    gammaField: clamp01(Number(fieldState.gammaField || 0)),
    liquidationField: clamp01(Number(fieldState.liquidationField || 0)),
    cascadeField: clamp01(Number(fieldState.cascadeField || 0)),
  };
}

function packHeatCells(cells = []) {
  const buffer = new Float32Array(MAX_HEAT_CELLS * 4);
  const count = Math.min(MAX_HEAT_CELLS, cells.length);
  for (let index = 0; index < count; index += 1) {
    const cell = cells[index];
    buffer[index * 4] = clamp01(Number(cell.x || 0) / 100);
    buffer[index * 4 + 1] = clamp01(Number(cell.y || 0) / 100);
    buffer[index * 4 + 2] = clamp01(Number(cell.intensity || 0));
    buffer[index * 4 + 3] = cell.side === "above" ? 1 : -1;
  }
  return { buffer, count };
}

function packGammaBands(bands = []) {
  const buffer = new Float32Array(MAX_GAMMA_BANDS * 4);
  const count = Math.min(MAX_GAMMA_BANDS, bands.length);
  for (let index = 0; index < count; index += 1) {
    const band = bands[index];
    buffer[index * 4] = clamp01(Number(band.y || 0) / 100);
    buffer[index * 4 + 1] = clamp01(Number(band.intensity || 0));
    buffer[index * 4 + 2] = band.role === "support" ? 1 : -1;
    buffer[index * 4 + 3] = 0;
  }
  return { buffer, count };
}

function packCascadePoints(points = []) {
  const buffer = new Float32Array(MAX_CASCADE_POINTS * 4);
  const count = Math.min(MAX_CASCADE_POINTS, points.length);
  for (let index = 0; index < count; index += 1) {
    const point = points[index];
    buffer[index * 4] = clamp01(Number(point.x || 0) / 100);
    buffer[index * 4 + 1] = clamp01(Number(point.y || 0) / 100);
    buffer[index * 4 + 2] = clamp01(Number(point.intensity || 0));
    buffer[index * 4 + 3] = clamp01(Number(point.size || 0) / 96);
  }
  return { buffer, count };
}

function createProgram(gl, vertexSource, fragmentSource) {
  const vertexShader = compileShader(gl, gl.VERTEX_SHADER, vertexSource);
  const fragmentShader = compileShader(gl, gl.FRAGMENT_SHADER, fragmentSource);
  if (!vertexShader || !fragmentShader) return null;

  const program = gl.createProgram();
  gl.attachShader(program, vertexShader);
  gl.attachShader(program, fragmentShader);
  gl.linkProgram(program);
  gl.deleteShader(vertexShader);
  gl.deleteShader(fragmentShader);

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    gl.deleteProgram(program);
    return null;
  }

  return program;
}

function compileShader(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    gl.deleteShader(shader);
    return null;
  }
  return shader;
}

function resizeCanvasToDisplaySize(canvas) {
  const dpr = window.devicePixelRatio || 1;
  const rect = canvas.getBoundingClientRect();
  const width = Math.max(1, Math.floor(rect.width * dpr));
  const height = Math.max(1, Math.floor(rect.height * dpr));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }
}

function clamp01(value) {
  if (!Number.isFinite(value)) return 0;
  return Math.min(1, Math.max(0, value));
}
