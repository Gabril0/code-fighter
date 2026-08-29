import * as THREE from "three";

const WIDTH = 1024;
const HEIGHT = 384;
const SPECTATOR_ACCENT = "#5ec8f5";
const SPECTATOR_PLATE = "rgba(253, 253, 253, 0.92)";
const SPECTATOR_INK = "#2a4d63";
const FADE_STEP = 0.06;

export const LABEL_ASPECT = HEIGHT / WIDTH;

export function createLabel(name, accent = SPECTATOR_ACCENT, crown = false) {
  const canvas = document.createElement("canvas");
  canvas.width = WIDTH;
  canvas.height = HEIGHT;

  const texture = new THREE.CanvasTexture(canvas);
  texture.magFilter = THREE.LinearFilter;
  texture.minFilter = THREE.LinearFilter;
  texture.generateMipmaps = false;

  const sprite = new THREE.Sprite(
    new THREE.SpriteMaterial({
      map: texture,
      transparent: true,
      depthWrite: false,
      depthTest: false,
      toneMapped: false,
    }),
  );

  const label = { canvas, texture, sprite, name, accent, crown, message: null, messageAlpha: 1 };
  paintLabel(label);
  return label;
}

export function paintLabel(label) {
  const { canvas, texture, name, accent, crown, message } = label;
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, WIDTH, HEIGHT);
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";

  const alpha = label.messageAlpha ?? 1;
  if (message && alpha > 0) {
    ctx.globalAlpha = alpha;
    const text = message.length > 40 ? `${message.slice(0, 39)}…` : message;
    ctx.font = "600 52px 'Fighter Body', system-ui, sans-serif";
    const width = Math.min(WIDTH - 40, ctx.measureText(text).width + 72);
    const x = (WIDTH - width) / 2;
    ctx.fillStyle = crown ? "rgba(9, 9, 11, 0.95)" : SPECTATOR_PLATE;
    ctx.strokeStyle = accent;
    ctx.lineWidth = crown ? 7 : 5;
    ctx.beginPath();
    ctx.roundRect(x, 16, width, 110, 26);
    ctx.fill();
    ctx.stroke();
    ctx.fillStyle = crown ? "#ffffff" : SPECTATOR_INK;
    ctx.fillText(text, WIDTH / 2, 73);
    ctx.globalAlpha = 1;
  }

  const plate = crown ? "700 68px" : "700 62px";
  ctx.font = `${plate} 'Fighter Body', system-ui, sans-serif`;
  const shown = name.length > 18 ? `${name.slice(0, 17)}…` : name;
  const nameWidth = ctx.measureText(shown).width + (crown ? 92 : 64);
  const plateHeight = crown ? 116 : 104;
  const top = crown ? 214 : 220;

  ctx.fillStyle = crown ? "rgba(9, 9, 11, 0.92)" : SPECTATOR_PLATE;
  ctx.strokeStyle = crown ? accent : accent;
  ctx.lineWidth = crown ? 6 : 4;
  ctx.beginPath();
  ctx.roundRect((WIDTH - nameWidth) / 2, top, nameWidth, plateHeight, 24);
  ctx.fill();
  ctx.stroke();

  if (crown) {
    ctx.fillStyle = accent;
    ctx.beginPath();
    ctx.roundRect((WIDTH - nameWidth) / 2 + 18, top + plateHeight / 2 - 9, 18, 18, 5);
    ctx.fill();
  }

  ctx.fillStyle = crown ? accent : SPECTATOR_INK;
  ctx.fillText(shown, WIDTH / 2 + (crown ? 12 : 0), top + plateHeight / 2 + 4);

  texture.needsUpdate = true;
}

export function sayOnLabel(label, message) {
  label.message = message;
  label.messageAlpha = 1;
  paintLabel(label);
}

export function fadeLabel(label, alpha) {
  const next = Math.max(0, Math.min(1, alpha));
  if (next > 0 && Math.abs(next - (label.messageAlpha ?? 1)) < FADE_STEP) return;
  label.messageAlpha = next;
  if (next <= 0) label.message = null;
  paintLabel(label);
}

export function disposeLabel(label) {
  label.texture.dispose();
  label.sprite.material.dispose();
}

export function outlineOf(root, color = 0x000000, thickness = 0.04) {
  const targets = [];
  root.traverse((child) => {
    if (!child.isMesh || child.userData.outlineShell) return;
    targets.push(child);
  });

  const created = [];
  for (const mesh of targets) {
    const material = new THREE.MeshBasicMaterial({
      color,
      side: THREE.BackSide,
      toneMapped: false,
    });
    material.onBeforeCompile = (shader) => {
      shader.uniforms.outlineThickness = { value: thickness };
      shader.vertexShader = `uniform float outlineThickness;\n${shader.vertexShader}`.replace(
        "#include <begin_vertex>",
        "#include <begin_vertex>\n\ttransformed += normalize(normal) * outlineThickness;",
      );
    };

    let shell;
    if (mesh.isSkinnedMesh) {
      shell = new THREE.SkinnedMesh(mesh.geometry, material);
      shell.bind(mesh.skeleton, mesh.bindMatrix);
    } else {
      shell = new THREE.Mesh(mesh.geometry, material);
    }

    shell.userData.outlineShell = true;
    shell.position.copy(mesh.position);
    shell.quaternion.copy(mesh.quaternion);
    shell.scale.copy(mesh.scale);
    shell.renderOrder = -1;
    shell.frustumCulled = false;
    mesh.parent.add(shell);
    created.push(shell);
  }
  return created;
}

export function tintOutline(root, color) {
  root.traverse((child) => {
    if (child.userData.outlineShell) child.material.color.setHex(color);
  });
}
