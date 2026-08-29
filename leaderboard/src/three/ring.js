import * as THREE from "three";

import { EffectComposer } from "three/examples/jsm/postprocessing/EffectComposer.js";
import { RenderPixelatedPass } from "three/examples/jsm/postprocessing/RenderPixelatedPass.js";
import { RenderPass } from "three/examples/jsm/postprocessing/RenderPass.js";
import { UnrealBloomPass } from "three/examples/jsm/postprocessing/UnrealBloomPass.js";
import { OutputPass } from "three/examples/jsm/postprocessing/OutputPass.js";

import {
  buildFloorBounce,
  buildSpotRig,
  floorTexture,
  ImpactFx,
  noiseTexture,
  softLight,
  THEME,
} from "./effects";
import {
  createLabel,
  disposeLabel,
  fadeLabel,
  LABEL_ASPECT,
  outlineOf,
  sayOnLabel,
  tintOutline,
} from "./labels";
import { toonGradient, toonify } from "./toon";
import { buildCrtPass } from "./crt";
import { buildSeatMap, Crowd } from "./crowd";
import { loadMiiSource, Mii } from "./mii";
import { STAGE_ASPECT, STAGE_HEIGHT, STAGE_WIDTH } from "../lib/stage";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";

export const MAX_ROSTER = 6;

const SIDE_SIGN = { left: -1, right: 1 };
const DEFAULT_COLOR = { left: 0x3b82f6, right: 0xef4444 };
const RING_HALF = 5.2;
const FLOOR_Y = 1.35;
const FIGHT_X = 1.28;
const KO_HOLD_MS = 1500;
const ENTER_MS = 1300;

const clamp01 = (t) => Math.min(1, Math.max(0, t));
const idleGap = () => RINGSIDE_IDLE_MIN_MS + Math.random() * RINGSIDE_IDLE_SPAN_MS;
const easeInOut = (t) => (t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2);

const RING_CANVAS_MESH = "Canvas";
const RING_MODEL_WIDTH = (RING_HALF + 0.9) * 2;
const PIXEL_SIZE = 2;
const SPAR_MIN_MS = 500;
const SPAR_VARIANCE_MS = 1000;
const SPAR_RECOVER_MS = 850;
const SPAR_CONTACT_MS = 440;
const SPAR_HIT_HOLD_MS = 700;
const SPAR_TURN_MS = 200;
const SPAR_COMBO_CHANCE = 0.38;
const SPAR_COMBO_GAP_MS = 320;
const SPAR_CLASH_CHANCE = 0.22;
const SPAR_CLASH_HOLD_MS = 900;
const FINISHER_RETRY_MS = 220;
const GLOVE_HEIGHT = 1.55;
const RINGSIDE_ACTIONS = [
  "Cheer", "Dance", "Dance2", "ShadowBox", "Anxious",
  "LookAround", "Scream", "ArmWave", "Walking",
];
const CELEBRATION_ACTIONS = ["Cheer", "Dance", "Dance2", "ArmWave", "Scream"];
const ATTACK_MOVES = [
  "Punch", "Kick", "DoublePunch", "Headbutt", "Tackle", "Shoryuken", "Haymaker",
];
const STRIKE_AMOUNT = { Headbutt: 0.12, Tackle: 0.2 };
const STRIKE_LIMBS = {
  Punch: ["forearm_R", "hand_R"],
  Kick: ["shin_R", "foot_R"],
  DoublePunch: ["forearm_R", "hand_R", "forearm_L", "hand_L"],
  Headbutt: ["head"],
  Tackle: ["shoulder_R", "shoulder_L"],
  Shoryuken: ["forearm_R", "hand_R"],
  Haymaker: ["forearm_R", "hand_R"],
};
const RINGSIDE_ACTION_MS = 3400;
const RINGSIDE_IDLE_MIN_MS = 5000;
const RINGSIDE_IDLE_SPAN_MS = 10000;
const RINGSIDE_PACE_MS = 1500;
const GHOST_INTERVAL_MS = 300000;
const GHOST_DURATION_MS = 500;
const GHOST_NOISE_MS = 70;
const RAGDOLL_PUSH = 11.6;
const RAGDOLL_LIFT = 6.4;
const RAGDOLL_MARGIN = 0.95;
const RAGDOLL_CLEARANCE = 0.34;
const FIGHTER_LABEL = 3.6;
const FIGHTER_LABEL_LIFT = 4.3;
const MISS_PUSH = 7.4;
const MISS_LIFT = 4.8;
const MISS_RECOVER_MS = 2600;
const FIGHTER_BUBBLE_MS = 7000;
const FIGHTER_BUBBLE_FADE_MS = 1400;
const GHOST_SEAT = new THREE.Vector3(-5.4, 0.05, -9.8);
const CORNER_X = RING_HALF + 2.1;
const CORNER_Z = 1.4;
const CORNER_GAP = 1.7;
const CORNER_Y = 0.05;
const CORNER_PACE_RANGE = 1.05;
const SHAKE_SPAN_MS = 420;
const SHAKE_FREQUENCY = 44;
const SHAKE_PUNCH = 0.26;
const SHAKE_STUMBLE = 0.34;
const SHAKE_KNOCKOUT = 0.55;
const SHAKE_CLASH = 0.14;
const FLASH_SPAN_MS = 110;
const FLASH_PUNCH = 0.3;
const FLASH_KNOCKOUT = 0.5;
const FINALE_DELAY_MS = 3000;
const FINALE_WALK_MS = 1800;
const CELEBRATE_SPREAD = 3.6;
const CELEBRATE_Z = 2.1;
const TROPHY_RISE_MS = 900;
const TROPHY_BOB = 0.22;
const TROPHY_SPIN = 0.7;
const TROPHY_HEIGHT = FLOOR_Y + 2.75;
const CONFETTI_MS = 1400;

function loadRingModel(scene) {
  return new GLTFLoader()
    .setPath("/models/")
    .loadAsync("ring.glb?v=3")
    .then((gltf) => {
      const model = gltf.scene;
      let canvasMesh = null;
      model.traverse((child) => {
        if (child.name === RING_CANVAS_MESH) canvasMesh = child;
      });
      const anchor = new THREE.Box3().setFromObject(canvasMesh ?? model);
      const anchorSize = anchor.getSize(new THREE.Vector3());
      const anchorCentre = anchor.getCenter(new THREE.Vector3());

      const scale = RING_MODEL_WIDTH / (Math.max(anchorSize.x, anchorSize.z) || 1);
      model.scale.setScalar(scale);
      model.position.x = -anchorCentre.x * scale;
      model.position.z = -anchorCentre.z * scale;
      model.position.y = FLOOR_Y - anchor.max.y * scale;
      model.traverse((child) => {
        if (child.isMesh) {
          child.receiveShadow = true;
          child.castShadow = true;
        }
      });
      toonify(model);
      scene.add(model);
      return model;
    });
}

function benchSlot(side, index) {
  const sign = SIDE_SIGN[side];
  return new THREE.Vector3(sign * (RING_HALF + 1.9), 0.05, -1.3 - index * 1.6);
}

function cornerSlot(side, index) {
  const sign = SIDE_SIGN[side];
  return new THREE.Vector3(sign * CORNER_X, CORNER_Y, CORNER_Z - index * CORNER_GAP);
}

function celebrateSpot(index, total) {
  const offset = (index - (total - 1) / 2) * CELEBRATE_SPREAD;
  return new THREE.Vector3(offset, FLOOR_Y, CELEBRATE_Z);
}

function buildTrophy() {
  const gold = new THREE.MeshToonMaterial({
    color: 0xf7cf46,
    emissive: 0x6b4a05,
    gradientMap: toonGradient(),
  });
  const plinth = new THREE.MeshToonMaterial({ color: 0x3a2a12, gradientMap: toonGradient() });

  const group = new THREE.Group();
  const cup = new THREE.Mesh(new THREE.CylinderGeometry(0.6, 0.26, 0.82, 20), gold);
  cup.position.y = 1.24;
  const lip = new THREE.Mesh(new THREE.TorusGeometry(0.6, 0.07, 10, 24), gold);
  lip.position.y = 1.65;
  lip.rotation.x = Math.PI / 2;
  const stem = new THREE.Mesh(new THREE.CylinderGeometry(0.11, 0.14, 0.44, 14), gold);
  stem.position.y = 0.61;
  const base = new THREE.Mesh(new THREE.CylinderGeometry(0.44, 0.54, 0.32, 20), plinth);
  base.position.y = 0.23;

  const handles = [-1, 1].map((sign) => {
    const handle = new THREE.Mesh(new THREE.TorusGeometry(0.26, 0.075, 8, 18, Math.PI), gold);
    handle.position.set(sign * 0.56, 1.32, 0);
    handle.rotation.z = sign * -Math.PI / 2;
    return handle;
  });

  [cup, lip, stem, base, ...handles].forEach((piece) => {
    piece.castShadow = true;
    group.add(piece);
  });
  return group;
}

function fightSpot(side) {
  return new THREE.Vector3(SIDE_SIGN[side] * FIGHT_X, FLOOR_Y, 0);
}

export class Ring {
  constructor(container) {
    this.container = container;
    this.clock = new THREE.Clock();
    this.disposed = false;

    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(0x04060a);
    this.scene.fog = new THREE.FogExp2(0x05070d, 0.012);

    this.camera = new THREE.PerspectiveCamera(45, 1, 0.1, 120);

    this.renderer = new THREE.WebGLRenderer({
      antialias: true,
      preserveDrawingBuffer: true,
    });
    this.renderer.setPixelRatio(1);
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.18;
    container.appendChild(this.renderer.domElement);

    this.scene.add(new THREE.HemisphereLight(0xa8bbdd, 0x2b3450, 0.95));
    this.scene.add(new THREE.AmbientLight(0x6c7ba3, 1.05));
    this.spotRig = buildSpotRig(this.scene, {
      colors: { left: DEFAULT_COLOR.left, right: DEFAULT_COLOR.right },
      fight: { left: fightSpot("left"), right: fightSpot("right") },
      corner: { left: cornerSlot("left", 0), right: cornerSlot("right", 0) },
    });

    const floor = new THREE.Mesh(
      new THREE.PlaneGeometry(90, 90),
      new THREE.MeshToonMaterial({
        color: 0x0a0e17,
        map: floorTexture(),
        gradientMap: toonGradient(),
      }),
    );
    floor.rotation.x = -Math.PI / 2;
    floor.receiveShadow = true;
    this.scene.add(floor);
    this.scene.add(buildFloorBounce());

    this.colors = { ...DEFAULT_COLOR };
    this.sides = {
      left: { roster: [], falls: 0, size: 0, pending: false },
      right: { roster: [], falls: 0, size: 0, pending: false },
    };

    this.resize();

    this.composer = new EffectComposer(this.renderer);
    this.pixelPass = new RenderPixelatedPass(PIXEL_SIZE, this.scene, this.camera, {
      normalEdgeStrength: 0.25,
      depthEdgeStrength: 0.15,
    });
    this.composer.addPass(this.pixelPass);
    this.flatPass = new RenderPass(this.scene, this.camera);
    this.flatPass.enabled = false;
    this.composer.addPass(this.flatPass);
    this.bloom = new UnrealBloomPass(new THREE.Vector2(1, 1), 0.14, 0.5, 0.95);
    this.composer.addPass(this.bloom);
    this.composer.addPass(new OutputPass());
    this.crtPass = buildCrtPass();
    this.composer.addPass(this.crtPass);
    this.effects = { pixelate: true, crt: true, aberration: true, bloom: true };

    this.fx = new ImpactFx(this.scene);
    this.buildScreenFlash();

    this.ghostLight = new THREE.PointLight(0xbfd4ff, 0, 22, 1.6);
    this.ghostLight.position.copy(GHOST_SEAT).add(new THREE.Vector3(0, 2.4, 1.6));
    this.scene.add(this.ghostLight);
    this.clockOffset = 0;
    this.viewerId = null;
    this.ghostVisible = false;
    this.ghostForcedUntil = 0;
    this.ghostNoiseClock = 0;
    this.overlay = new THREE.Scene();
    this.crowd = new Crowd(
      this.scene,
      buildSeatMap([GHOST_SEAT]),
      new THREE.Vector3(0, FLOOR_Y + 1, 0),
      this.overlay,
    );
    this.shake = null;
    this.screenFlashLife = 0;
    this.screenFlashPeak = 0;
    this.sparTimer = SPAR_MIN_MS;
    this.sparSide = "left";
    this.outcome = null;
    this.finaleTimer = null;
    this.trophy = null;
    this.trophyClock = 0;
    this.confettiTimer = 0;
    this.renderer.setAnimationLoop(() => this.update());

    this.rosters = { left: [], right: [] };
    this.ready = Promise.all([loadRingModel(this.scene), loadMiiSource()]).then(
      ([, source]) => {
        this.miiSource = source;
      },
    );
  }

  setEffects(effects) {
    if (!effects) return;
    this.effects = { ...this.effects, ...effects };
    this.pixelPass.enabled = this.effects.pixelate;
    this.bloom.enabled = this.effects.bloom;
    this.flatPass.enabled = !this.effects.pixelate;
    this.crtPass.uniforms.curvature.value = this.effects.crt ? 1 : 0;
    this.crtPass.uniforms.aberration.value = this.effects.aberration ? 1 : 0;
    this.crtPass.enabled = this.effects.crt || this.effects.aberration;
  }

  setColors({ left, right }) {
    for (const [side, value] of [["left", left], ["right", right]]) {
      if (!Number.isFinite(value)) continue;
      this.colors[side] = value;
      this.sides[side].roster.forEach((fighter) => fighter.setColor(value));
      const slot = side === "left" ? 0 : 1;
      const corner = this.spotRig?.spots[2 + slot];
      if (corner) corner.color.copy(softLight(value));
      const beam = this.spotRig?.beams[2 + slot];
      if (beam) beam.material.color.setHex(value);
      const rim = this.spotRig?.rims[slot];
      if (rim) rim.color.copy(softLight(value));
    }
  }

  setRoster(members) {
    this.pendingRoster = members;
    if (!this.miiSource) {
      this.ready.then(() => {
        if (!this.disposed && this.pendingRoster === members) this.applyRoster(members);
      });
      return;
    }
    this.applyRoster(members);
  }

  applyRoster(members) {
    for (const side of ["left", "right"]) {
      const roster = (members?.[side] ?? []).slice(0, MAX_ROSTER);
      if (roster.length === 0) continue;

      const state = this.sides[side];
      state.roster.forEach((fighter) => {
        this.scene.remove(fighter.root);
        if (fighter.labelData) {
          this.overlay.remove(fighter.labelData.sprite);
          disposeLabel(fighter.labelData);
        }
        fighter.dispose();
      });
      state.roster = roster.map(
        (member) =>
          new Mii({
            source: this.miiSource,
            member,
            color: this.colors[side],
            side,
            sideSign: SIDE_SIGN[side],
          }),
      );
      state.roster.forEach((fighter, index) => {
        const person = roster[index];
        const accent = `#${this.colors[side].toString(16).padStart(6, "0")}`;
        fighter.labelData = createLabel(person?.name ?? "fighter", accent, true);
        fighter.labelData.sprite.scale.set(FIGHTER_LABEL, FIGHTER_LABEL * LABEL_ASPECT, 1);
        fighter.memberId = person?.id ?? null;
        this.overlay.add(fighter.labelData.sprite);
        outlineOf(fighter.pivot, fighter.memberId === this.viewerId ? 0xffffff : 0x0a0a0c, 0.045);
        this.scene.add(fighter.root);
      });
      state.size = state.roster.length;
      this.seat(side, state.falls);
    }
  }

  active(side) {
    const state = this.sides[side];
    return state.size ? state.falls % state.size : 0;
  }

  queuePlace(side, index) {
    const state = this.sides[side];
    return (index - this.active(side) + state.size) % state.size - 1;
  }

  seat(side, falls) {
    const state = this.sides[side];
    state.falls = Math.max(0, falls);
    state.pending = false;
    const active = this.active(side);

    state.roster.forEach((fighter, index) => {
      fighter.timers = [];
      fighter.move = null;
      fighter.fade = null;
      fighter.turn = null;
      fighter.stopRagdoll();
      fighter.setOpacity(1);

      if (index === active) {
        fighter.placeAt(fightSpot(side));
        fighter.faceCenter();
        fighter.play("Idle");
      } else {
        fighter.placeAt(cornerSlot(side, this.queuePlace(side, index)));
        fighter.faceCenter();
        fighter.play("Idle");
        fighter.ringsideBusy = false;
        fighter.ringsideTimer = idleGap() * Math.random();
      }
    });
  }

  setFalls(side, falls) {
    const state = this.sides[side];
    if (!state.size) {
      state.falls = falls;
      return;
    }
    const gap = falls - state.falls;
    if (gap === 0) return;
    if (gap === 1) {
      state.pending = true;
      this.sparTimer = Math.min(this.sparTimer, FINISHER_RETRY_MS);
      return;
    }
    this.seat(side, falls);
  }

  awaitingFall() {
    if (this.sides.left.pending) return "left";
    if (this.sides.right.pending) return "right";
    return null;
  }

  knockOut(side) {
    const state = this.sides[side];
    const victim = state.roster[this.active(side)];
    if (!victim) return;

    state.pending = false;
    state.falls += 1;
    const active = this.active(side);

    victim.play("Death", { loop: false, fade: 0.12 });
    victim.launch(
      new THREE.Vector3(SIDE_SIGN[side] * RAGDOLL_PUSH, RAGDOLL_LIFT, -RAGDOLL_PUSH * 0.35),
      {
        minX: -RING_HALF + RAGDOLL_MARGIN,
        maxX: RING_HALF - RAGDOLL_MARGIN,
        minZ: -RING_HALF + RAGDOLL_MARGIN,
        maxZ: RING_HALF - RAGDOLL_MARGIN,
        floor: FLOOR_Y + RAGDOLL_CLEARANCE,
      },
    );
    this.fx?.burst("knockout", fightSpot(side).clone().setY(FLOOR_Y + 0.6), THEME.impact, {
      direction: new THREE.Vector3(SIDE_SIGN[side], 0.75, 0.25).normalize(),
    });
    this.shakeCamera(SHAKE_KNOCKOUT);
    this.flashScreen(FLASH_KNOCKOUT);

    const corner = cornerSlot(side, this.queuePlace(side, state.roster.indexOf(victim)));
    victim.after(KO_HOLD_MS, () => {
      victim.stopRagdoll();
      victim.setOpacity(1);
      victim.facePoint(corner);
      victim.play("Walking", { fade: 0.2 });
      victim.moveTo(corner, ENTER_MS, () => {
        victim.faceCenter();
        victim.play("Idle", { fade: 0.2 });
        victim.ringsideBusy = false;
        victim.ringsideTimer = idleGap();
      });
    });

    state.roster.forEach((waiting, index) => {
      if (index === active || waiting === victim) return;
      waiting.moveTo(cornerSlot(side, this.queuePlace(side, index)), ENTER_MS);
    });

    const next = state.roster[active];
    if (!next || next === victim) return;
    next.timers = [];
    next.move = null;
    next.facePoint(fightSpot(side));
    next.play("Walking", { fade: 0.15 });
    next.moveTo(fightSpot(side), ENTER_MS, () => {
      next.faceCenter();
      next.play("Idle");
    });
  }

  setOutcome(winner) {
    if (this.outcome === (winner ?? null)) return;
    this.outcome = winner ?? null;
    clearTimeout(this.finaleTimer);
    this.finaleTimer = null;

    if (!this.outcome) {
      this.hideTrophy();
      for (const side of ["left", "right"]) this.seat(side, this.sides[side].falls);
      return;
    }

    this.finaleTimer = setTimeout(() => {
      if (this.disposed || !this.outcome) return;
      this.stageFinale(this.outcome);
    }, FINALE_DELAY_MS);
  }

  restageOutcome() {
    if (!this.outcome) return;
    this.stageFinale(this.outcome);
  }

  stageFinale(winner) {
    const loser = winner === "left" ? "right" : "left";
    const champions = this.sides[winner].roster;
    this.sides.left.pending = false;
    this.sides.right.pending = false;

    champions.forEach((fighter, index) => {
      const spot = celebrateSpot(index, champions.length);
      fighter.timers = [];
      fighter.stopRagdoll();
      fighter.setOpacity(1);
      fighter.facePoint(spot);
      fighter.play("Walking", { fade: 0.2 });
      fighter.moveTo(spot, FINALE_WALK_MS, () => {
        fighter.faceCamera();
        fighter.play("Cheer", { fade: 0.25 });
        fighter.ringsideBusy = true;
        fighter.ringsideTimer = RINGSIDE_ACTION_MS;
      });
    });

    this.sides[loser].roster.forEach((fighter, index) => {
      const bench = benchSlot(loser, index);
      fighter.timers = [];
      fighter.stopRagdoll();
      fighter.setOpacity(1);
      fighter.facePoint(bench);
      fighter.play("Walking", { fade: 0.2 });
      fighter.moveTo(bench, FINALE_WALK_MS, () => {
        fighter.faceCenter();
        fighter.play("Sitting", { fade: 0.3 });
      });
    });

    this.showTrophy();
  }

  showTrophy() {
    if (!this.trophy) {
      this.trophy = buildTrophy();
      this.scene.add(this.trophy);
      this.trophyLight = new THREE.PointLight(0xffd978, 7, 11, 2.1);
      this.scene.add(this.trophyLight);
    }
    this.trophy.visible = true;
    this.trophyClock = 0;
    this.confettiTimer = 0;
    this.fx?.burst("knockout", new THREE.Vector3(0, TROPHY_HEIGHT, 0), THEME.gold);
  }

  hideTrophy() {
    if (!this.trophy) return;
    this.trophy.visible = false;
    if (this.trophyLight) this.trophyLight.intensity = 0;
  }

  driftTrophy(delta) {
    if (!this.trophy?.visible) return;
    this.trophyClock += delta;
    const rise = clamp01((this.trophyClock * 1000) / TROPHY_RISE_MS);
    const lift = easeInOut(rise) * TROPHY_HEIGHT;
    this.trophy.position.set(0, lift + Math.sin(this.trophyClock * 1.6) * TROPHY_BOB * rise, 0);
    this.trophy.rotation.y += delta * TROPHY_SPIN;
    if (this.trophyLight) {
      this.trophyLight.position.set(0, this.trophy.position.y + 1.6, 0);
      this.trophyLight.intensity = 5.5 + Math.sin(this.trophyClock * 3.2) * 2;
    }

    this.confettiTimer -= delta * 1000;
    if (this.confettiTimer > 0) return;
    this.confettiTimer = CONFETTI_MS;
    const champions = this.sides[this.outcome]?.roster ?? [];
    const cheered = champions[Math.floor(Math.random() * champions.length)];
    const spot = (cheered?.root.position.clone() ?? new THREE.Vector3()).setY(FLOOR_Y + 2.4);
    this.fx?.burst("solve", spot, this.colors[this.outcome] ?? THEME.gold);
    this.fx?.pixel(new THREE.Vector3(0, this.trophy.position.y + 0.9, 0), THEME.gold, 2.4, 0.5);
  }

  stumble(side) {
    const state = this.sides[side];
    const fighter = state.roster[this.active(side)];
    if (!fighter) return;

    fighter.timers = [];
    fighter.move = null;
    fighter.play("Death", { loop: false, fade: 0.1 });
    fighter.launch(
      new THREE.Vector3(SIDE_SIGN[side] * MISS_PUSH, MISS_LIFT, -MISS_PUSH * 0.3),
      {
        minX: -RING_HALF + RAGDOLL_MARGIN,
        maxX: RING_HALF - RAGDOLL_MARGIN,
        minZ: -RING_HALF + RAGDOLL_MARGIN,
        maxZ: RING_HALF - RAGDOLL_MARGIN,
        floor: FLOOR_Y + RAGDOLL_CLEARANCE,
      },
    );
    this.fx?.burst("hit", fightSpot(side).clone().setY(FLOOR_Y + 1.2), THEME.impact, {
      direction: new THREE.Vector3(SIDE_SIGN[side], 0.6, 0.2).normalize(),
    });
    this.shakeCamera(SHAKE_STUMBLE);

    this.sparTimer = Math.max(this.sparTimer, MISS_RECOVER_MS + 600);
    fighter.after(MISS_RECOVER_MS, () => {
      fighter.stopRagdoll();
      fighter.placeAt(fightSpot(side));
      fighter.faceCenter();
      fighter.play("Idle", { fade: 0.25 });
    });
  }

  punch(side) {
    const state = this.sides[side];
    const fighter = state.roster[this.active(side)];
    if (!fighter) return;
    fighter.play("Punch", { loop: false, fade: 0.1 });
    fighter.after(900, () => fighter.play("Idle", { fade: 0.2 }));

    this.fx?.burst("solve", fightSpot(side).clone().setY(FLOOR_Y + 0.08), THEME.brand);

    const impact = new THREE.Vector3(0, FLOOR_Y + 1.5, 0);
    setTimeout(() => {
      this.fx?.burst("punch", impact, this.colors[side], {
        direction: new THREE.Vector3(-SIDE_SIGN[side], 0.4, 0.2).normalize(),
      });
      this.shakeCamera(SHAKE_PUNCH);
      this.flashScreen(FLASH_PUNCH);
    }, 180);
  }

  buildScreenFlash() {
    this.flashScene = new THREE.Scene();
    this.flashCamera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
    this.flashQuad = new THREE.Mesh(
      new THREE.PlaneGeometry(2, 2),
      new THREE.MeshBasicMaterial({
        color: 0xffffff,
        transparent: true,
        opacity: 0,
        depthTest: false,
        depthWrite: false,
      }),
    );
    this.flashScene.add(this.flashQuad);
  }

  shakeCamera(amount) {
    if (this.shake && this.shake.amount > amount) return;
    this.shake = { amount, elapsed: 0, seed: Math.random() * 100 };
  }

  flashScreen(peak) {
    this.screenFlashPeak = Math.max(this.screenFlashPeak, peak);
    this.screenFlashLife = FLASH_SPAN_MS;
  }

  driftCamera(delta) {
    if (!this.cameraHome) return;
    if (!this.shake) return;

    this.shake.elapsed += delta * 1000;
    const t = clamp01(this.shake.elapsed / SHAKE_SPAN_MS);
    if (t >= 1) {
      this.shake = null;
      this.camera.position.copy(this.cameraHome);
      this.camera.lookAt(this.cameraFocus);
      return;
    }

    const decay = (1 - t) ** 2 * this.shake.amount;
    const phase = this.shake.seed + this.shake.elapsed / 1000 * SHAKE_FREQUENCY;
    this.camera.position.set(
      this.cameraHome.x + Math.sin(phase) * decay,
      this.cameraHome.y + Math.sin(phase * 1.37 + 1.1) * decay * 0.75,
      this.cameraHome.z + Math.sin(phase * 0.79 + 2.3) * decay * 0.4,
    );
    this.camera.lookAt(this.cameraFocus);
  }

  renderScreenFlash(delta) {
    if (this.screenFlashLife <= 0) return;
    this.screenFlashLife -= delta * 1000;
    const remaining = clamp01(this.screenFlashLife / FLASH_SPAN_MS);
    this.flashQuad.material.opacity = remaining * this.screenFlashPeak;
    if (remaining <= 0) {
      this.screenFlashPeak = 0;
      return;
    }
    this.renderer.render(this.flashScene, this.flashCamera);
  }

  resize() {
    this.camera.aspect = STAGE_ASPECT;

    const halfFov = THREE.MathUtils.degToRad(this.camera.fov) / 2;
    const forWidth = (RING_HALF + 4.6) / (Math.tan(halfFov) * STAGE_ASPECT);
    const forHeight = 5.6 / Math.tan(halfFov);
    const distance = Math.max(forWidth, forHeight);

    this.camera.position.set(0, 5.4 + distance * 0.16, distance);
    this.cameraHome = this.camera.position.clone();
    this.cameraFocus = new THREE.Vector3(0, FLOOR_Y + 1.1, 0);
    this.camera.lookAt(this.cameraFocus);
    this.camera.updateProjectionMatrix();

    this.renderer.setSize(STAGE_WIDTH, STAGE_HEIGHT);
    this.composer?.setSize(STAGE_WIDTH, STAGE_HEIGHT);
    this.pixelPass?.setSize(STAGE_WIDTH, STAGE_HEIGHT);
  }

  spar(delta) {
    const falling = this.awaitingFall();
    if (this.outcome && !falling) return;

    this.sparTimer -= delta * 1000;
    if (this.sparTimer > 0) return;

    const left = this.sides.left.roster[this.active('left')];
    const right = this.sides.right.roster[this.active('right')];
    if (!left || !right) return;

    if (left.state !== "Idle" || right.state !== "Idle") {
      this.sparTimer = falling ? FINISHER_RETRY_MS : SPAR_MIN_MS + Math.random() * SPAR_VARIANCE_MS;
      return;
    }

    this.sparTimer = SPAR_MIN_MS + Math.random() * SPAR_VARIANCE_MS;

    if (falling) {
      this.sparSide = falling === "left" ? "right" : "left";
      const scorer = falling === "left" ? right : left;
      const doomed = falling === "left" ? left : right;
      this.throwPunch(scorer, doomed, false, true);
      return;
    }

    if (Math.random() < SPAR_CLASH_CHANCE) {
      this.clash(left, right);
      return;
    }

    this.sparSide = Math.random() < 0.5 ? "left" : "right";
    const attacker = this.sparSide === "left" ? left : right;
    const defender = this.sparSide === "left" ? right : left;
    const combo = Math.random() < SPAR_COMBO_CHANCE;

    this.throwPunch(attacker, defender, combo);
  }

  throwPunch(attacker, defender, combo, finisher = false) {
    const move = ATTACK_MOVES[Math.floor(Math.random() * ATTACK_MOVES.length)];
    attacker.turnTo(attacker.engageAngle(), SPAR_TURN_MS);
    attacker.play(move, { fade: 0.08 });
    attacker.strike(STRIKE_LIMBS[move] ?? STRIKE_LIMBS.Punch, { amount: STRIKE_AMOUNT[move] });

    const land = () => {
      if (defender.state !== "Idle" && defender.state !== "Hit") return;
      if (finisher) {
        this.fx?.pixel(this.contactPoint(attacker, defender), THEME.white, 2.4, 0.45);
        this.knockOut(defender.side);
        return;
      }
      defender.replay("Hit", { fade: 0.05 });
      const contact = this.contactPoint(attacker, defender);
      const direction = new THREE.Vector3()
        .subVectors(defender.root.position, attacker.root.position)
        .setY(0.55)
        .normalize();
      this.fx?.burst("hit", contact, this.colors[defender.side === "left" ? "right" : "left"], {
        direction,
      });
      this.fx?.pixel(contact, THEME.white, 1.5);
      defender.after(SPAR_HIT_HOLD_MS, () => {
        if (defender.state === "Hit") defender.play("Idle", { fade: 0.2 });
      });
    };

    attacker.after(SPAR_CONTACT_MS, land);

    if (combo) {
      attacker.after(SPAR_COMBO_GAP_MS + SPAR_CONTACT_MS, () => {
        if (!ATTACK_MOVES.includes(attacker.state)) return;
        const next = ATTACK_MOVES[Math.floor(Math.random() * ATTACK_MOVES.length)];
        attacker.replay(next);
        attacker.strike(STRIKE_LIMBS[next] ?? STRIKE_LIMBS.Punch, { amount: STRIKE_AMOUNT[next] });
        attacker.after(SPAR_CONTACT_MS, land);
      });
    }

    const recover = SPAR_RECOVER_MS + (combo ? SPAR_COMBO_GAP_MS + SPAR_CONTACT_MS : 0);
    attacker.after(recover, () => {
      if (!ATTACK_MOVES.includes(attacker.state)) return;
      attacker.play("Idle", { fade: 0.18 });
      attacker.turnTo(attacker.guardAngle(), SPAR_TURN_MS + 120);
    });
  }

  clash(left, right) {
    for (const fighter of [left, right]) {
      fighter.turnTo(fighter.engageAngle(), SPAR_TURN_MS);
      fighter.play("Punch", { fade: 0.08 });
      fighter.strike(STRIKE_LIMBS.Punch);
      fighter.after(SPAR_CONTACT_MS, () => {
        if (fighter.state !== "Punch") return;
        fighter.play("Clash", { fade: 0.04 });
      });
      fighter.after(SPAR_CONTACT_MS + SPAR_CLASH_HOLD_MS, () => {
        if (fighter.state !== "Clash") return;
        fighter.play("Idle", { fade: 0.2 });
        fighter.turnTo(fighter.guardAngle(), SPAR_TURN_MS + 120);
      });
    }

    const midpoint = new THREE.Vector3()
      .addVectors(left.root.position, right.root.position)
      .multiplyScalar(0.5)
      .setY(FLOOR_Y + GLOVE_HEIGHT);
    left.after(SPAR_CONTACT_MS, () => {
      this.fx?.pixel(midpoint, THEME.gold, 2.6, 0.5);
      this.fx?.pixel(midpoint, THEME.white, 1.7, 0.36);
      this.shakeCamera(SHAKE_CLASH);
    });

    this.sparTimer = Math.max(this.sparTimer, SPAR_CLASH_HOLD_MS + 500);
  }

  ringside(fighter, home, delta, actions, facingCamera = false, gap = idleGap) {
    if (fighter.move || fighter.ragdoll || fighter.timers.length) return;
    if (!Number.isFinite(fighter.ringsideTimer)) {
      fighter.ringsideTimer = (gap ? gap() : RINGSIDE_ACTION_MS) * Math.random();
    }
    fighter.ringsideTimer -= delta * 1000;
    if (fighter.ringsideTimer > 0) return;

    const settle = () => (facingCamera ? fighter.faceCamera() : fighter.faceCenter());

    if (gap && fighter.ringsideBusy) {
      fighter.ringsideBusy = false;
      fighter.ringsideTimer = gap();
      fighter.placeAt(home);
      settle();
      fighter.play("Idle", { fade: 0.35 });
      return;
    }

    fighter.ringsideBusy = true;
    fighter.ringsideTimer = RINGSIDE_ACTION_MS;
    const action = actions[Math.floor(Math.random() * actions.length)];

    if (action === "Walking") {
      const target = home.clone();
      target.z += (Math.random() < 0.5 ? -1 : 1) * CORNER_PACE_RANGE;
      fighter.facePoint(target);
      fighter.play("Walking", { fade: 0.2 });
      fighter.moveTo(target, RINGSIDE_PACE_MS, () => {
        fighter.facePoint(home);
        fighter.moveTo(home, RINGSIDE_PACE_MS, () => {
          settle();
          fighter.play("Standing", { fade: 0.25 });
        });
      });
      return;
    }

    fighter.placeAt(home);
    if (action === "Still") fighter.facePoint(this.camera.position);
    else settle();
    fighter.play(action, { fade: 0.3 });
  }

  spawnGhost() {
    if (this.ghost || !this.miiSource) return;
    this.ghost = new Mii({
      source: this.miiSource,
      member: null,
      color: 0x11161f,
      side: "left",
      sideSign: -1,
    });
    this.ghost.setFace(noiseTexture());
    this.ghost.root.visible = false;
    this.ghost.placeAt(GHOST_SEAT);
    this.ghost.play("Sitting");
    this.scene.add(this.ghost.root);
  }

  syncClock(serverNow) {
    if (!Number.isFinite(serverNow)) return;
    this.clockOffset = serverNow - Date.now();
  }

  haunt(delta) {
    this.spawnGhost();
    if (!this.ghost) return;

    const shared = Date.now() + (this.clockOffset ?? 0);
    const phase = ((shared % GHOST_INTERVAL_MS) + GHOST_INTERVAL_MS) % GHOST_INTERVAL_MS;
    const summoned = Date.now() < (this.ghostForcedUntil ?? 0);
    const due = summoned || phase < GHOST_DURATION_MS;

    if (due && !this.ghostVisible) {
      this.ghostVisible = true;
      this.ghost.root.visible = true;
      this.ghost.facePoint(this.camera.position);
    } else if (!due && this.ghostVisible) {
      this.ghostVisible = false;
      this.ghost.root.visible = false;
      this.ghostLight.intensity = 0;
    }

    if (!this.ghostVisible) return;

    this.ghost.update(delta);
    this.ghostNoiseClock += delta * 1000;
    if (this.ghostNoiseClock >= GHOST_NOISE_MS) {
      this.ghostNoiseClock = 0;
      noiseTexture(96, this.ghost.faceTexture);
    }
    this.ghostLight.intensity = Math.random() < 0.25 ? 0 : 8 + Math.random() * 22;
  }

  setClock(serverNow) {
    this.syncClock(serverNow);
  }

  setViewer(id) {
    if (this.viewerId === id) return;
    this.viewerId = id;
    this.crowd.setViewer(id);
    for (const state of Object.values(this.sides)) {
      state.roster.forEach((fighter) => {
        tintOutline(fighter.pivot, fighter.memberId === id ? 0xffffff : 0x0a0a0c);
      });
    }
  }

  fighterSays(memberId, message) {
    for (const side of Object.values(this.sides)) {
      const fighter = side.roster.find((one) => one.memberId === memberId);
      if (!fighter?.labelData) continue;
      sayOnLabel(fighter.labelData, message);
      fighter.labelBubble = FIGHTER_BUBBLE_MS;
    }
  }

  setSpectators(people) {
    this.crowd.sync(people ?? []);
  }

  spectatorSays(id, message) {
    this.crowd.say(id, message);
  }

  spectatorEmotes(id, emote) {
    this.crowd.emote(id, emote);
  }

  seatCount() {
    return this.crowd.seats.length;
  }

  previewClip(side, clip) {
    const state = this.sides[side];
    const fighter = state.roster[this.active(side)];
    if (!fighter) return;
    fighter.timers = [];
    fighter.move = null;
    fighter.stopRagdoll();
    fighter.placeAt(fightSpot(side));

    if (ATTACK_MOVES.includes(clip)) {
      fighter.turnTo(fighter.engageAngle(), SPAR_TURN_MS);
      fighter.replay(clip, { fade: 0.05 });
      fighter.strike(STRIKE_LIMBS[clip] ?? STRIKE_LIMBS.Punch, { amount: STRIKE_AMOUNT[clip] });
      this.fx?.pixel(fightSpot(side).clone().setY(FLOOR_Y + GLOVE_HEIGHT), THEME.white, 1.5);
    } else if (clip === "Death") {
      fighter.faceCenter();
      fighter.replay(clip, { fade: 0.05 });
      fighter.launch(
        new THREE.Vector3(SIDE_SIGN[side] * RAGDOLL_PUSH, RAGDOLL_LIFT, -RAGDOLL_PUSH * 0.35),
        {
          minX: -RING_HALF + RAGDOLL_MARGIN,
          maxX: RING_HALF - RAGDOLL_MARGIN,
          minZ: -RING_HALF + RAGDOLL_MARGIN,
          maxZ: RING_HALF - RAGDOLL_MARGIN,
          floor: FLOOR_Y + RAGDOLL_CLEARANCE,
        },
      );
    } else if (clip === "Still") {
      fighter.facePoint(this.camera.position);
      fighter.replay(clip, { fade: 0.05 });
    } else {
      fighter.faceCenter();
      fighter.replay(clip, { fade: 0.05 });
    }

    this.sparTimer = Math.max(this.sparTimer, 4000);
    fighter.after(3600, () => {
      fighter.stopRagdoll();
      fighter.placeAt(fightSpot(side));
      fighter.faceCenter();
      fighter.play("Idle", { fade: 0.25 });
    });
  }

  previewBurst(kind, side) {
    const spot = fightSpot(side).clone().setY(FLOOR_Y + GLOVE_HEIGHT);
    this.fx?.burst(kind, spot, this.colors[side], {
      direction: new THREE.Vector3(-SIDE_SIGN[side], 0.45, 0.2).normalize(),
    });
    this.shakeCamera(kind === "knockout" ? SHAKE_KNOCKOUT : SHAKE_PUNCH);
    this.flashScreen(kind === "knockout" ? FLASH_KNOCKOUT : FLASH_PUNCH);
  }

  previewGhost() {
    this.spawnGhost();
    if (!this.ghost) return;
    this.ghostForcedUntil = Date.now() + GHOST_DURATION_MS;
  }

  contactPoint(attacker, defender) {
    return new THREE.Vector3()
      .copy(attacker.root.position)
      .lerp(defender.root.position, 0.66)
      .setY(FLOOR_Y + GLOVE_HEIGHT);
  }

  update() {
    const delta = this.clock.getDelta();
    this.spar(delta);
    this.haunt(delta);
    this.crowd.update(delta);

    this.driftTrophy(delta);

    if (this.outcome && this.trophy?.visible) {
      const champions = this.sides[this.outcome].roster;
      champions.forEach((fighter, index) => {
        this.ringside(
          fighter,
          celebrateSpot(index, champions.length),
          delta,
          CELEBRATION_ACTIONS,
          true,
          null,
        );
      });
    } else if (!this.outcome) {
      for (const side of ["left", "right"]) {
        const state = this.sides[side];
        state.roster.forEach((fighter, index) => {
          if (index === this.active(side)) return;
          this.ringside(fighter, cornerSlot(side, this.queuePlace(side, index)), delta, RINGSIDE_ACTIONS);
        });
      }
    }
    for (const side of Object.values(this.sides)) {
      side.roster.forEach((fighter, index) => {
        fighter.update(delta);
        if (!fighter.labelData) return;
        if (fighter.labelBubble > 0) {
          fighter.labelBubble -= delta * 1000;
          if (fighter.labelBubble <= 0) sayOnLabel(fighter.labelData, null);
          else if (fighter.labelBubble < FIGHTER_BUBBLE_FADE_MS) {
            fadeLabel(fighter.labelData, fighter.labelBubble / FIGHTER_BUBBLE_FADE_MS);
          }
        }
        fighter.labelData.sprite.visible = true;
        fighter.labelData.sprite.position.set(
          fighter.root.position.x,
          fighter.root.position.y + FIGHTER_LABEL_LIFT,
          fighter.root.position.z,
        );
      });
    }
    this.fx.update(delta);
    this.driftCamera(delta);
    this.composer.render(delta);
    this.renderer.autoClear = false;
    this.renderer.clearDepth();
    this.renderer.render(this.overlay, this.camera);
    this.renderScreenFlash(delta);
    this.renderer.autoClear = true;
  }

  dispose() {
    this.disposed = true;
    this.renderer.setAnimationLoop(null);
    clearTimeout(this.finaleTimer);
    this.crowd?.dispose();
    this.flashQuad?.geometry.dispose();
    this.flashQuad?.material.dispose();

    for (const state of Object.values(this.sides)) {
      state.roster.forEach((fighter) => {
        this.scene.remove(fighter.root);
        if (fighter.labelData) {
          this.overlay.remove(fighter.labelData.sprite);
          disposeLabel(fighter.labelData);
        }
        fighter.dispose();
      });
      state.roster = [];
      state.size = 0;
    }

    this.scene.traverse((object) => {
      if (object.geometry) object.geometry.dispose();
      if (object.material) {
        const materials = Array.isArray(object.material) ? object.material : [object.material];
        materials.forEach((material) => material.dispose());
      }
    });
    this.renderer.dispose();
    this.renderer.domElement.remove();
  }
}
