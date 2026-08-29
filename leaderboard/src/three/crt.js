import { ShaderPass } from "three/examples/jsm/postprocessing/ShaderPass.js";

const ABERRATION_SPREAD = 0.0065;
const CURVE = 0.014;

const shader = {
  uniforms: {
    tDiffuse: { value: null },
    curvature: { value: 1 },
    aberration: { value: 1 },
  },
  vertexShader: /* glsl */ `
    varying vec2 vUv;
    void main() {
      vUv = uv;
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }
  `,
  fragmentShader: /* glsl */ `
    uniform sampler2D tDiffuse;
    uniform float curvature;
    uniform float aberration;
    varying vec2 vUv;

    vec2 bulge(vec2 uv) {
      vec2 centred = uv * 2.0 - 1.0;
      centred *= 1.0 + dot(centred.yx, centred.yx) * ${CURVE.toFixed(4)};
      return centred * 0.5 + 0.5;
    }

    void main() {
      vec2 uv = mix(vUv, bulge(vUv), curvature);

      if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);
        return;
      }

      vec2 fromCentre = uv - 0.5;
      vec2 shift = fromCentre * ${ABERRATION_SPREAD.toFixed(5)} * length(fromCentre) * 2.0 * aberration;

      gl_FragColor = vec4(
        texture2D(tDiffuse, uv + shift).r,
        texture2D(tDiffuse, uv).g,
        texture2D(tDiffuse, uv - shift).b,
        1.0
      );
    }
  `,
};

export function buildCrtPass() {
  return new ShaderPass(shader);
}
