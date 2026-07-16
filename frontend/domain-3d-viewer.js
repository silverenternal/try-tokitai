(function initializeAtlasDomain3D() {
  "use strict";

  const mounted = new WeakMap();

  function color(name, fallback) {
    const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    return value || fallback;
  }

  function finitePoint(value) {
    if (!Array.isArray(value) || value.length < 3) return null;
    const point = value.slice(0, 3).map(Number);
    return point.every(Number.isFinite) ? point : null;
  }

  class Domain3DViewer {
    constructor(canvas, geometry, options = {}) {
      if (!(canvas instanceof HTMLCanvasElement)) throw new TypeError("A canvas element is required");
      this.canvas = canvas;
      this.context = canvas.getContext("2d", { alpha: true, desynchronized: true });
      this.options = options;
      this.points = (geometry?.points || []).map(finitePoint).filter(Boolean).slice(0, 30000);
      this.faces = (geometry?.faces || [])
        .filter((face) => Array.isArray(face) && face.length >= 2)
        .map((face) => face.map(Number).filter((index) => Number.isInteger(index) && index >= 0 && index < this.points.length))
        .filter((face) => face.length >= 2)
        .slice(0, 10000);
      this.center = [0, 0, 0];
      this.radius = 1;
      this.rotation = { yaw: -0.72, pitch: -0.42 };
      this.pan = { x: 0, y: 0 };
      this.zoom = 1;
      this.drag = null;
      this.frame = 0;
      this.computeBounds();
      this.bind();
      this.resizeObserver = new ResizeObserver(() => this.resize());
      this.resizeObserver.observe(canvas);
      this.resize();
    }

    computeBounds() {
      if (!this.points.length) return;
      const min = [Infinity, Infinity, Infinity];
      const max = [-Infinity, -Infinity, -Infinity];
      for (const point of this.points) {
        for (let axis = 0; axis < 3; axis += 1) {
          min[axis] = Math.min(min[axis], point[axis]);
          max[axis] = Math.max(max[axis], point[axis]);
        }
      }
      this.center = min.map((value, axis) => (value + max[axis]) / 2);
      this.radius = Math.max(1e-9, Math.hypot(max[0] - min[0], max[1] - min[1], max[2] - min[2]) / 2);
      this.bounds = { min, max };
    }

    bind() {
      this.onPointerDown = (event) => {
        if (event.button !== 0 && event.button !== 1) return;
        event.preventDefault();
        this.drag = {
          pointerId: event.pointerId,
          x: event.clientX,
          y: event.clientY,
          yaw: this.rotation.yaw,
          pitch: this.rotation.pitch,
          panX: this.pan.x,
          panY: this.pan.y,
          mode: event.shiftKey || event.button === 1 ? "pan" : "rotate",
        };
        this.canvas.setPointerCapture(event.pointerId);
        this.canvas.classList.add("is-dragging");
      };
      this.onPointerMove = (event) => {
        if (!this.drag || this.drag.pointerId !== event.pointerId) return;
        const dx = event.clientX - this.drag.x;
        const dy = event.clientY - this.drag.y;
        if (this.drag.mode === "pan") {
          this.pan.x = this.drag.panX + dx;
          this.pan.y = this.drag.panY + dy;
        } else {
          this.rotation.yaw = this.drag.yaw + dx * 0.009;
          this.rotation.pitch = Math.max(-Math.PI * 0.49, Math.min(Math.PI * 0.49, this.drag.pitch + dy * 0.009));
        }
        this.requestDraw();
      };
      this.onPointerUp = (event) => {
        if (this.drag?.pointerId !== event.pointerId) return;
        this.drag = null;
        this.canvas.classList.remove("is-dragging");
      };
      this.onWheel = (event) => {
        event.preventDefault();
        this.zoom = Math.max(0.15, Math.min(12, this.zoom * Math.exp(-event.deltaY * 0.0013)));
        this.requestDraw();
      };
      this.onDoubleClick = () => this.fit();
      this.canvas.addEventListener("pointerdown", this.onPointerDown);
      this.canvas.addEventListener("pointermove", this.onPointerMove);
      this.canvas.addEventListener("pointerup", this.onPointerUp);
      this.canvas.addEventListener("pointercancel", this.onPointerUp);
      this.canvas.addEventListener("wheel", this.onWheel, { passive: false });
      this.canvas.addEventListener("dblclick", this.onDoubleClick);
    }

    resize() {
      const bounds = this.canvas.getBoundingClientRect();
      const ratio = Math.min(2, Math.max(1, window.devicePixelRatio || 1));
      const width = Math.max(1, Math.round(bounds.width * ratio));
      const height = Math.max(1, Math.round(bounds.height * ratio));
      if (this.canvas.width !== width || this.canvas.height !== height) {
        this.canvas.width = width;
        this.canvas.height = height;
      }
      this.pixelRatio = ratio;
      this.width = Math.max(1, bounds.width);
      this.height = Math.max(1, bounds.height);
      this.requestDraw();
    }

    fit() {
      this.rotation = { yaw: -0.72, pitch: -0.42 };
      this.pan = { x: 0, y: 0 };
      this.zoom = 1;
      this.requestDraw();
    }

    requestDraw() {
      if (this.frame) return;
      this.frame = requestAnimationFrame(() => {
        this.frame = 0;
        this.draw();
      });
    }

    rotate(point) {
      const x = (point[0] - this.center[0]) / this.radius;
      const y = (point[1] - this.center[1]) / this.radius;
      const z = (point[2] - this.center[2]) / this.radius;
      const cosY = Math.cos(this.rotation.yaw);
      const sinY = Math.sin(this.rotation.yaw);
      const x1 = x * cosY + z * sinY;
      const z1 = -x * sinY + z * cosY;
      const cosX = Math.cos(this.rotation.pitch);
      const sinX = Math.sin(this.rotation.pitch);
      return [x1, y * cosX - z1 * sinX, y * sinX + z1 * cosX];
    }

    project(point) {
      const camera = 3.4;
      const perspective = camera / Math.max(0.35, camera - point[2]);
      const scale = Math.min(this.width, this.height) * 0.38 * this.zoom * perspective;
      return {
        x: this.width / 2 + this.pan.x + point[0] * scale,
        y: this.height / 2 + this.pan.y - point[1] * scale,
        z: point[2],
        perspective,
      };
    }

    drawGrid(context) {
      const size = Math.min(this.width, this.height) * 0.31 * this.zoom;
      const originX = this.width / 2 + this.pan.x;
      const originY = this.height / 2 + this.pan.y;
      context.save();
      context.strokeStyle = color("--line-soft", "rgba(128,128,128,.18)");
      context.lineWidth = 1;
      context.globalAlpha = 0.6;
      for (let step = -5; step <= 5; step += 1) {
        const offset = (step / 5) * size;
        context.beginPath();
        context.moveTo(originX - size, originY + offset * 0.32);
        context.lineTo(originX + size, originY + offset * 0.32);
        context.stroke();
      }
      context.restore();
    }

    drawAxes(context) {
      const origin = this.project(this.rotate(this.center));
      const length = this.radius * 0.42;
      const axes = [
        { end: [this.center[0] + length, this.center[1], this.center[2]], color: "#d45d55", label: "X" },
        { end: [this.center[0], this.center[1] + length, this.center[2]], color: "#56a86b", label: "Y" },
        { end: [this.center[0], this.center[1], this.center[2] + length], color: "#5b82d6", label: "Z" },
      ];
      context.save();
      context.font = "10px system-ui";
      for (const axis of axes) {
        const end = this.project(this.rotate(axis.end));
        context.strokeStyle = axis.color;
        context.fillStyle = axis.color;
        context.lineWidth = 1.5;
        context.beginPath();
        context.moveTo(origin.x, origin.y);
        context.lineTo(end.x, end.y);
        context.stroke();
        context.fillText(axis.label, end.x + 4, end.y - 4);
      }
      context.restore();
    }

    draw() {
      if (!this.context) return;
      const context = this.context;
      const ratio = this.pixelRatio || 1;
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      context.clearRect(0, 0, this.width, this.height);
      this.drawGrid(context);
      if (!this.points.length) return;
      const rotated = this.points.map((point) => this.rotate(point));
      const projected = rotated.map((point) => this.project(point));
      const accent = color("--accent", "#e98748");
      const stroke = color("--text-3", "#87909a");
      const faceFill = color("--domain-3d-face", "rgba(233,135,72,.14)");
      const faceRecords = this.faces.map((face) => ({
        face,
        depth: face.reduce((total, index) => total + (rotated[index]?.[2] || 0), 0) / face.length,
      })).sort((left, right) => left.depth - right.depth);

      if (faceRecords.length) {
        for (const { face, depth } of faceRecords) {
          const screen = face.map((index) => projected[index]).filter(Boolean);
          if (screen.length < 2) continue;
          context.beginPath();
          screen.forEach((point, index) => index ? context.lineTo(point.x, point.y) : context.moveTo(point.x, point.y));
          if (screen.length >= 3) context.closePath();
          const light = Math.max(0.38, Math.min(0.95, 0.68 + depth * 0.12));
          context.fillStyle = faceFill;
          context.strokeStyle = stroke;
          context.globalAlpha = light;
          context.lineWidth = 0.75;
          if (screen.length >= 3) context.fill();
          context.stroke();
        }
      } else {
        context.fillStyle = accent;
        context.globalAlpha = 0.76;
        for (const point of projected.slice(0, 16000)) {
          const radius = Math.max(0.7, Math.min(2.4, 1.1 * point.perspective));
          context.beginPath();
          context.arc(point.x, point.y, radius, 0, Math.PI * 2);
          context.fill();
        }
      }
      context.globalAlpha = 1;
      this.drawAxes(context);
      context.fillStyle = color("--text-3", "#87909a");
      context.font = "10px system-ui";
      context.fillText(`${this.points.length.toLocaleString()} vertices · ${this.faces.length.toLocaleString()} faces`, 12, 20);
      context.fillText("Drag rotate · Shift+drag pan · Wheel zoom · Double-click fit", 12, this.height - 14);
    }

    dispose() {
      if (this.frame) cancelAnimationFrame(this.frame);
      this.resizeObserver?.disconnect();
      this.canvas.removeEventListener("pointerdown", this.onPointerDown);
      this.canvas.removeEventListener("pointermove", this.onPointerMove);
      this.canvas.removeEventListener("pointerup", this.onPointerUp);
      this.canvas.removeEventListener("pointercancel", this.onPointerUp);
      this.canvas.removeEventListener("wheel", this.onWheel);
      this.canvas.removeEventListener("dblclick", this.onDoubleClick);
      mounted.delete(this.canvas);
    }
  }

  function mount(canvas, geometry, options) {
    mounted.get(canvas)?.dispose();
    const viewer = new Domain3DViewer(canvas, geometry, options);
    mounted.set(canvas, viewer);
    return viewer;
  }

  function unmount(canvas) {
    mounted.get(canvas)?.dispose();
  }

  window.AtlasDomain3D = Object.freeze({ mount, unmount });
})();
