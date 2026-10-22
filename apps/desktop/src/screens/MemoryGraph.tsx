import { useEffect, useRef, useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import * as THREE from 'three'
import type { MemoryGraphPayload, MemoryNodeDto } from './types'

export function MemoryGraph() {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const [data, setData] = useState<MemoryGraphPayload | null>(null)
  const [loading, setLoading] = useState(true)
  const [selectedNode, setSelectedNode] = useState<MemoryNodeDto | null>(null)
  const [cameraMode, setCameraMode] = useState<'cinema' | 'interactive' | 'fast'>('cinema')
  const [isAudioEnabled, setIsAudioEnabled] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')
  const [nodeCount, setNodeCount] = useState(0)
  const [axonCount, setAxonCount] = useState(0)
  const [camCoords, setCamCoords] = useState('X: 0 Y: 45 Z: 290')

  const audioCtxRef = useRef<AudioContext | null>(null)
  const nodeMeshesRef = useRef<THREE.Group[]>([])
  const sparksRef = useRef<{ mesh: THREE.Mesh; curve: THREE.Curve<THREE.Vector3>; progress: number; speed: number }[]>([])
  const targetCamPosRef = useRef<THREE.Vector3 | null>(null)
  const targetLookAtRef = useRef<THREE.Vector3 | null>(null)
  const sceneRef = useRef<THREE.Scene | null>(null)
  const cameraRef = useRef<THREE.PerspectiveCamera | null>(null)

  const playTone = useCallback((freq: number, type: OscillatorType = 'sine', duration = 0.35, gainLevel = 0.12) => {
    if (!isAudioEnabled) return
    try {
      if (!audioCtxRef.current) {
        audioCtxRef.current = new (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)()
      }
      const ctx = audioCtxRef.current
      if (ctx.state === 'suspended') ctx.resume()
      const osc = ctx.createOscillator()
      const gain = ctx.createGain()
      osc.type = type
      osc.frequency.setValueAtTime(freq, ctx.currentTime)
      osc.frequency.exponentialRampToValueAtTime(freq * 1.5, ctx.currentTime + duration)

      gain.gain.setValueAtTime(gainLevel, ctx.currentTime)
      gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + duration)

      osc.connect(gain)
      gain.connect(ctx.destination)
      osc.start()
      osc.stop(ctx.currentTime + duration)
    } catch {}
  }, [isAudioEnabled])

  // Load Initial Memory Graph
  useEffect(() => {
    async function fetchGraph() {
      try {
        const res = await invoke<MemoryGraphPayload>('get_memory_graph')
        setData(res)
      } catch (err) {
        console.warn('Backend get_memory_graph failed, using simulated matrix:', err)
        setData({
          nodes: [
            { id: 'gov-1', label: 'Master Governance', kind: 'core', narrative: 'Dual-Zone Master Prompt with cryptographic SHA-256 integrity seal.', files: ['SYNAPSE.md'], ts: Date.now(), val: 0.998 },
            { id: 'mem-1', label: 'Identity & Invariants', kind: 'core', narrative: 'Non-negotiable clinical behavioral invariants and ethical constraints.', files: ['IDENTITY.md'], ts: Date.now(), val: 0.984 },
            { id: 'gate-1', label: 'Sensory Gating', kind: 'observation', narrative: 'Dynamic context compression filter delivering 42% token reduction.', files: ['compression.rs'], ts: Date.now(), val: 0.972 },
            { id: 'gate-2', label: 'Context Folding', kind: 'observation', narrative: 'Reversible contextual folding of raw tool outputs and logs.', files: ['spool.rs'], ts: Date.now(), val: 0.946 },
            { id: 'obs-1', label: 'TDD Rigor Protocol', kind: 'project', narrative: 'Strict test-first development invariant with Vitest and Cargo.', files: ['tests/'], ts: Date.now(), val: 0.921 },
            { id: 'obs-2', label: 'Clean Architecture', kind: 'project', narrative: 'Ports and adapters separation between CLI, desktop GUI and core.', files: ['packages/core/'], ts: Date.now(), val: 0.915 },
            { id: 'diag-1', label: 'Doctor Diagnostics', kind: 'discovery', narrative: 'Rule 15 integrity engine detecting missing/tampered master prompt.', files: ['doctor.rs'], ts: Date.now(), val: 0.967 },
            { id: 'sync-1', label: 'Universal Sync Mesh', kind: 'discovery', narrative: 'Real-time multi-agent sync engine connecting 15 modern LLM tools.', files: ['sync.rs'], ts: Date.now(), val: 0.953 },
            { id: 'mem-2', label: 'Episodic Memory Matrix', kind: 'core', narrative: 'Permanent vector-indexed memory clusters stored in AI BRAIN.', files: ['mem.rs'], ts: Date.now(), val: 0.989 },
            { id: 'intake-1', label: 'Clinical Intake Wizard', kind: 'change', narrative: 'Interactive elicitation wizard for stack, architecture, and demeanor.', files: ['onboard.rs'], ts: Date.now(), val: 0.960 }
          ],
          edges: [
            { source: 'gov-1', target: 'mem-1', label: 'governs' },
            { source: 'gov-1', target: 'gate-1', label: 'gates' },
            { source: 'gov-1', target: 'diag-1', label: 'verifies' },
            { source: 'gov-1', target: 'sync-1', label: 'syncs' },
            { source: 'gate-1', target: 'gate-2', label: 'folds' },
            { source: 'gate-1', target: 'mem-2', label: 'indexes' },
            { source: 'mem-1', target: 'obs-1', label: 'enforces' },
            { source: 'mem-1', target: 'obs-2', label: 'structures' },
            { source: 'diag-1', target: 'intake-1', label: 'provisions' },
            { source: 'sync-1', target: 'obs-2', label: 'projects' },
            { source: 'intake-1', target: 'gov-1', label: 'initializes' }
          ],
          total_observations: 10
        })
      } finally {
        setLoading(false)
      }
    }
    fetchGraph()
  }, [])

  // Three.js WebGL Engine Lifecycle
  useEffect(() => {
    if (!containerRef.current || !data) return
    const container = containerRef.current
    let width = container.clientWidth
    let height = container.clientHeight

    const scene = new THREE.Scene()
    scene.fog = new THREE.FogExp2(0x020408, 0.0018)
    sceneRef.current = scene

    const camera = new THREE.PerspectiveCamera(55, width / height, 1, 3000)
    camera.position.set(0, 45, 290)
    cameraRef.current = camera

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: 'high-performance' })
    renderer.setSize(width, height)
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
    renderer.toneMapping = THREE.ACESFilmicToneMapping
    renderer.toneMappingExposure = 1.25
    container.innerHTML = ''
    container.appendChild(renderer.domElement)

    // Lighting
    const ambientLight = new THREE.AmbientLight(0x0a192f, 2.0)
    scene.add(ambientLight)
    const centerLight = new THREE.PointLight(0x38bdf8, 3.5, 400)
    centerLight.position.set(0, 0, 0)
    scene.add(centerLight)

    // Cyber Ground Grid
    const gridHelper = new THREE.GridHelper(500, 30, 0x0ea5e9, 0x1e293b)
    gridHelper.position.y = -110
    ;(gridHelper.material as THREE.Material).opacity = 0.25
    ;(gridHelper.material as THREE.Material).transparent = true
    scene.add(gridHelper)

    // Parallax Dust / Starfield
    const dustCount = 1800
    const dustGeo = new THREE.BufferGeometry()
    const dustPositions = new Float32Array(dustCount * 3)
    const dustColors = new Float32Array(dustCount * 3)
    for (let i = 0; i < dustCount; i++) {
      dustPositions[i * 3] = (Math.random() - 0.5) * 800
      dustPositions[i * 3 + 1] = (Math.random() - 0.5) * 600
      dustPositions[i * 3 + 2] = (Math.random() - 0.5) * 800
      const isGold = Math.random() > 0.85
      dustColors[i * 3] = isGold ? 0.96 : 0.22
      dustColors[i * 3 + 1] = isGold ? 0.75 : 0.74
      dustColors[i * 3 + 2] = isGold ? 0.15 : 0.97
    }
    dustGeo.setAttribute('position', new THREE.BufferAttribute(dustPositions, 3))
    dustGeo.setAttribute('color', new THREE.BufferAttribute(dustColors, 3))
    const dustMat = new THREE.PointsMaterial({
      size: 2.0,
      vertexColors: true,
      transparent: true,
      opacity: 0.75,
      blending: THREE.AdditiveBlending
    })
    const dustParticles = new THREE.Points(dustGeo, dustMat)
    scene.add(dustParticles)

    // Singularity Accretion Ring
    const ringGeo = new THREE.RingGeometry(35, 37, 64)
    const ringMat = new THREE.MeshBasicMaterial({
      color: 0x38bdf8,
      side: THREE.DoubleSide,
      transparent: true,
      opacity: 0.35,
      blending: THREE.AdditiveBlending
    })
    const accretionRing = new THREE.Mesh(ringGeo, ringMat)
    accretionRing.rotation.x = Math.PI / 2
    scene.add(accretionRing)

    // Category Color Mapping
    const getKindColor = (kind: string) => {
      switch (kind) {
        case 'core': return 0xf59e0b
        case 'observation': return 0x06b6d4
        case 'project': return 0x10b981
        case 'discovery': return 0xef4444
        case 'change': return 0x0ea5e9
        default: return 0x8b5cf6
      }
    }

    // Node Meshes Group
    const nodeGroup = new THREE.Group()
    scene.add(nodeGroup)
    const meshes: THREE.Group[] = []

    const count = data.nodes.length
    data.nodes.forEach((node, idx) => {
      const group = new THREE.Group()
      const radius = 6 + (node.val * 3.5)
      const color = getKindColor(node.kind)

      // Golden Spiral Layout for aesthetic 3D positioning
      const phi = Math.acos(-1 + (2 * idx) / count)
      const theta = Math.sqrt(count * Math.PI) * phi
      const dist = 75 + (idx % 3) * 20
      const x = dist * Math.cos(theta) * Math.sin(phi)
      const y = dist * Math.sin(theta) * Math.sin(phi) * 0.7
      const z = dist * Math.cos(phi)
      group.position.set(x, y, z)

      // Inner Glowing Core
      const sphereGeo = new THREE.SphereGeometry(radius, 24, 24)
      const sphereMat = new THREE.MeshStandardMaterial({
        color: color,
        emissive: color,
        emissiveIntensity: 1.8,
        roughness: 0.2,
        metalness: 0.8
      })
      const coreMesh = new THREE.Mesh(sphereGeo, sphereMat)
      group.add(coreMesh)

      // Outer Wireframe Icosahedron
      const icoGeo = new THREE.IcosahedronGeometry(radius * 1.5, 1)
      const icoMat = new THREE.MeshBasicMaterial({
        color: color,
        wireframe: true,
        transparent: true,
        opacity: 0.65,
        blending: THREE.AdditiveBlending
      })
      const icoMesh = new THREE.Mesh(icoGeo, icoMat)
      group.add(icoMesh)

      // Gyroscopic Orbital Ring
      const ringGeo = new THREE.TorusGeometry(radius * 1.8, 0.35, 8, 36)
      const ringMat = new THREE.MeshBasicMaterial({
        color: color,
        transparent: true,
        opacity: 0.8,
        blending: THREE.AdditiveBlending
      })
      const gyroRing = new THREE.Mesh(ringGeo, ringMat)
      gyroRing.rotation.x = Math.random() * Math.PI
      gyroRing.rotation.y = Math.random() * Math.PI
      group.add(gyroRing)

      if (radius >= 8) {
        const pLight = new THREE.PointLight(color, 1.2, radius * 6)
        group.add(pLight)
      }

      group.userData = { node, coreMesh, icoMesh, gyroRing }
      nodeGroup.add(group)
      meshes.push(group)
    })
    nodeMeshesRef.current = meshes
    setNodeCount(meshes.length)

    // Curved 3D Axons & Sparks
    const axonGroup = new THREE.Group()
    scene.add(axonGroup)
    const localSparks: { mesh: THREE.Mesh; curve: THREE.Curve<THREE.Vector3>; progress: number; speed: number }[] = []

    data.edges.forEach(edge => {
      const srcMesh = meshes.find(m => m.userData.node.id === edge.source)
      const tgtMesh = meshes.find(m => m.userData.node.id === edge.target)
      if (!srcMesh || !tgtMesh) return

      const p1 = srcMesh.position.clone()
      const p2 = tgtMesh.position.clone()
      const mid = new THREE.Vector3().addVectors(p1, p2).multiplyScalar(0.5)
      mid.add(mid.clone().normalize().multiplyScalar(15))

      const curve = new THREE.QuadraticBezierCurve3(p1, mid, p2)
      const points = curve.getPoints(32)
      const axonGeo = new THREE.BufferGeometry().setFromPoints(points)
      const axonMat = new THREE.LineBasicMaterial({
        color: 0x38bdf8,
        transparent: true,
        opacity: 0.28,
        blending: THREE.AdditiveBlending
      })
      axonGroup.add(new THREE.Line(axonGeo, axonMat))

      // Spark
      const sparkMesh = new THREE.Mesh(
        new THREE.SphereGeometry(1.2, 8, 8),
        new THREE.MeshBasicMaterial({
          color: srcMesh.userData.node.kind === 'core' ? 0xf59e0b : 0x38bdf8,
          transparent: true,
          opacity: 0.95,
          blending: THREE.AdditiveBlending
        })
      )
      scene.add(sparkMesh)
      localSparks.push({
        mesh: sparkMesh,
        curve: curve,
        progress: Math.random(),
        speed: 0.003 + Math.random() * 0.004
      })
    })
    sparksRef.current = localSparks
    setAxonCount(data.edges.length)

    // Raycasting & Mouse Interaction
    const raycaster = new THREE.Raycaster()
    const mouse = new THREE.Vector2(-1000, -1000)
    let hoveredGroup: THREE.Group | null = null

    const onMouseMove = (e: MouseEvent) => {
      const rect = container.getBoundingClientRect()
      mouse.x = ((e.clientX - rect.left) / width) * 2 - 1
      mouse.y = -((e.clientY - rect.top) / height) * 2 + 1

      raycaster.setFromCamera(mouse, camera)
      const interactiveMeshes = meshes.map(m => m.userData.coreMesh)
      const hits = raycaster.intersectObjects(interactiveMeshes)

      if (hits.length > 0) {
        const hitGroup = hits[0].object.parent as THREE.Group
        if (hoveredGroup !== hitGroup) {
          if (hoveredGroup) hoveredGroup.userData.icoMesh.scale.set(1, 1, 1)
          hoveredGroup = hitGroup
          hoveredGroup.userData.icoMesh.scale.set(1.25, 1.25, 1.25)
          playTone(720, 'sine', 0.2, 0.08)
          container.style.cursor = 'pointer'
        }
      } else {
        if (hoveredGroup) {
          hoveredGroup.userData.icoMesh.scale.set(1, 1, 1)
          hoveredGroup = null
        }
        container.style.cursor = 'grab'
      }
    }

    const onClick = (e: MouseEvent) => {
      const rect = container.getBoundingClientRect()
      mouse.x = ((e.clientX - rect.left) / width) * 2 - 1
      mouse.y = -((e.clientY - rect.top) / height) * 2 + 1

      raycaster.setFromCamera(mouse, camera)
      const interactiveMeshes = meshes.map(m => m.userData.coreMesh)
      const hits = raycaster.intersectObjects(interactiveMeshes)
      if (hits.length > 0) {
        const grp = hits[0].object.parent as THREE.Group
        const node = grp.userData.node as MemoryNodeDto
        setSelectedNode(node)
        playTone(440, 'triangle', 0.45, 0.18)

        // Smooth camera fly-to
        targetCamPosRef.current = grp.position.clone().add(new THREE.Vector3(0, 10, 60))
        targetLookAtRef.current = grp.position.clone()
      }
    }

    // Manual Orbit Dragging
    let isDragging = false
    let prevMouseX = 0
    let prevMouseY = 0

    const onMouseDown = (e: MouseEvent) => {
      if (e.button === 0) {
        isDragging = true
        prevMouseX = e.clientX
        prevMouseY = e.clientY
      }
    }

    const onMouseUp = () => {
      isDragging = false
    }

    const onDragMove = (e: MouseEvent) => {
      if (!isDragging) return
      const deltaX = e.clientX - prevMouseX
      const deltaY = e.clientY - prevMouseY
      prevMouseX = e.clientX
      prevMouseY = e.clientY

      const rotSpeed = 0.005
      const spherical = new THREE.Spherical().setFromVector3(camera.position)
      spherical.theta -= deltaX * rotSpeed
      spherical.phi = Math.max(0.1, Math.min(Math.PI - 0.1, spherical.phi - deltaY * rotSpeed))
      camera.position.setFromSpherical(spherical)
      camera.lookAt(0, 0, 0)
    }

    const onWheel = (e: WheelEvent) => {
      e.preventDefault()
      const zoomFactor = e.deltaY * 0.15
      const dir = camera.position.clone().normalize()
      const newPos = camera.position.clone().add(dir.multiplyScalar(zoomFactor))
      if (newPos.length() > 40 && newPos.length() < 700) {
        camera.position.copy(newPos)
      }
    }

    container.addEventListener('mousemove', onMouseMove)
    container.addEventListener('click', onClick)
    container.addEventListener('mousedown', onMouseDown)
    window.addEventListener('mouseup', onMouseUp)
    window.addEventListener('mousemove', onDragMove)
    container.addEventListener('wheel', onWheel, { passive: false })

    // Animation Loop
    let animId: number
    const clock = new THREE.Clock()

    const animate = () => {
      animId = requestAnimationFrame(animate)
      const time = clock.getElapsedTime()

      // Auto-rotate if cinema mode active
      if (cameraMode === 'cinema' && !isDragging && !targetCamPosRef.current) {
        const spherical = new THREE.Spherical().setFromVector3(camera.position)
        spherical.theta += 0.003
        spherical.phi = (Math.PI / 2.3) + Math.sin(time * 0.4) * 0.15
        camera.position.setFromSpherical(spherical)
        camera.lookAt(0, 0, 0)
      }

      // Rotate ground grid & dust
      gridHelper.rotation.y += 0.0006
      dustParticles.rotation.y -= 0.0003
      accretionRing.rotation.z += 0.002

      // Animate Nodes
      meshes.forEach((grp, i) => {
        if (grp.userData.icoMesh) {
          grp.userData.icoMesh.rotation.x += 0.012
          grp.userData.icoMesh.rotation.y += 0.015
        }
        if (grp.userData.gyroRing) {
          grp.userData.gyroRing.rotation.x -= 0.018
          grp.userData.gyroRing.rotation.z += 0.01
        }
        const breathe = 1 + Math.sin(time * 2 + i) * 0.03
        if (grp.userData.coreMesh) {
          grp.userData.coreMesh.scale.set(breathe, breathe, breathe)
        }
      })

      // Animate Sparks along Axons
      localSparks.forEach(s => {
        s.progress += s.speed
        if (s.progress > 1) s.progress = 0
        const p = s.curve.getPoint(s.progress)
        s.mesh.position.copy(p)
      })

      // Smooth Camera Transitions
      if (targetCamPosRef.current && targetLookAtRef.current) {
        camera.position.lerp(targetCamPosRef.current, 0.06)
        camera.lookAt(targetLookAtRef.current)
        if (camera.position.distanceTo(targetCamPosRef.current) < 1.0) {
          targetCamPosRef.current = null
          targetLookAtRef.current = null
        }
      }

      const cc = camera.position
      setCamCoords(`X: ${cc.x.toFixed(0)} Y: ${cc.y.toFixed(0)} Z: ${cc.z.toFixed(0)}`)
      renderer.render(scene, camera)
    }

    animate()

    const onResize = () => {
      if (!container) return
      width = container.clientWidth
      height = container.clientHeight
      camera.aspect = width / height
      camera.updateProjectionMatrix()
      renderer.setSize(width, height)
    }
    window.addEventListener('resize', onResize)

    return () => {
      cancelAnimationFrame(animId)
      window.removeEventListener('resize', onResize)
      container.removeEventListener('mousemove', onMouseMove)
      container.removeEventListener('click', onClick)
      container.removeEventListener('mousedown', onMouseDown)
      window.removeEventListener('mouseup', onMouseUp)
      window.removeEventListener('mousemove', onDragMove)
      container.removeEventListener('wheel', onWheel)
      renderer.dispose()
    }
  }, [data, cameraMode, playTone])

  // Real-time Search Filter
  useEffect(() => {
    const q = searchQuery.toLowerCase().trim()
    nodeMeshesRef.current.forEach(grp => {
      const node = grp.userData.node as MemoryNodeDto
      const match = !q || node.label.toLowerCase().includes(q) || node.kind.toLowerCase().includes(q) || node.narrative.toLowerCase().includes(q)
      grp.visible = match
    })
  }, [searchQuery])

  return (
    <div className="flex h-full flex-col bg-[#020408] text-slate-100 overflow-hidden font-mono select-none relative">
      {/* Top Header Deck */}
      <header className="flex flex-wrap items-center justify-between border-b border-slate-800/80 bg-[#060a12]/90 px-6 py-3 z-20 backdrop-blur-xl">
        <div className="flex items-center gap-5">
          <div className="flex items-center gap-3">
            <div className="relative flex h-3 w-3">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-cyan-400 opacity-75"></span>
              <span className="relative inline-flex rounded-full h-3 w-3 bg-cyan-400 shadow-[0_0_10px_#22d3ee]"></span>
            </div>
            <span className="font-bold text-sm tracking-[0.25em] text-cyan-400">
              SYNAPSE // 3D NEURAL CINEMA
            </span>
          </div>
          <div className="hidden md:flex items-center gap-3 text-xs text-slate-400 border-l border-slate-800 pl-4">
            <span>NODES: <span className="text-white font-semibold">{nodeCount}</span></span>
            <span>·</span>
            <span>AXONS: <span className="text-amber-400 font-semibold">{axonCount}</span></span>
            <span>·</span>
            <span className="text-emerald-400 font-semibold flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span> 60 FPS
            </span>
          </div>
        </div>

        {/* Toolbar */}
        <div className="flex items-center gap-2.5 text-xs">
          <input
            type="text"
            placeholder="Search synaptic memory..."
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            className="bg-[#0c1322] border border-slate-800 rounded-lg px-3 py-1.5 text-xs text-white placeholder-slate-500 focus:border-cyan-400 focus:outline-none w-44 transition focus:w-56"
          />

          <button
            onClick={() => {
              setIsAudioEnabled(!isAudioEnabled)
              playTone(580, 'triangle', 0.4, 0.15)
            }}
            className={`border px-3 py-1.5 rounded-lg flex items-center gap-1.5 transition ${
              isAudioEnabled
                ? 'border-cyan-400 text-white bg-cyan-950/40 shadow-[0_0_12px_rgba(56,189,248,0.4)]'
                : 'border-slate-800 text-slate-400 bg-[#0c1322]'
            }`}
          >
            <span>{isAudioEnabled ? '🔊 Audio: ON' : '🔇 Audio: OFF'}</span>
          </button>

          <select
            value={cameraMode}
            onChange={e => setCameraMode(e.target.value as 'cinema' | 'interactive' | 'fast')}
            className="border border-slate-800 bg-[#0c1322] px-3 py-1.5 rounded-lg text-slate-200 focus:border-cyan-400 focus:outline-none"
          >
            <option value="cinema">Cinema Drone 3D</option>
            <option value="interactive">Interactive Orbit</option>
          </select>

          <button
            onClick={() => {
              if (cameraRef.current) {
                targetCamPosRef.current = new THREE.Vector3(0, 45, 290)
                targetLookAtRef.current = new THREE.Vector3(0, 0, 0)
                playTone(640, 'sine', 0.3, 0.12)
              }
            }}
            className="border border-slate-800 bg-[#0c1322] px-3 py-1.5 rounded-lg text-cyan-400 hover:border-cyan-400 hover:text-white transition"
          >
            Reset View
          </button>
        </div>
      </header>

      {/* Main 3D Container */}
      <div className="relative flex-1 w-full h-full">
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-[#020408]/80 z-30">
            <span className="text-cyan-400 text-sm tracking-widest animate-pulse">SYNTHESIZING 3D NEURAL SPACE...</span>
          </div>
        )}

        <div ref={containerRef} className="w-full h-full cursor-grab active:cursor-grabbing" />

        {/* Telemetry Stamp */}
        <div className="pointer-events-none absolute top-4 left-6 z-10 text-[11px] text-slate-500 space-y-1">
          <div>COORDINATES: <span className="text-cyan-400">{camCoords}</span></div>
          <div>FOV: <span className="text-white">55°</span> · SHADING: <span className="text-amber-400">VOLUMETRIC PBR</span></div>
        </div>

        {/* Node Holographic Drawer */}
        {selectedNode && (
          <div className="absolute right-6 top-6 bottom-20 w-80 bg-[#060a12]/90 border border-slate-800 rounded-xl p-5 z-20 flex flex-col justify-between backdrop-blur-xl shadow-2xl">
            <div>
              <div className="flex items-center justify-between border-b border-slate-800 pb-3 mb-4">
                <span className="px-2 py-0.5 text-[10px] font-bold rounded uppercase tracking-wider bg-cyan-950/60 text-cyan-400 border border-cyan-800/50">
                  {selectedNode.kind}
                </span>
                <button
                  onClick={() => setSelectedNode(null)}
                  className="text-slate-400 hover:text-white text-xs"
                >
                  ✕ CLOSE
                </button>
              </div>

              <h2 className="text-base font-bold text-white mb-1.5">{selectedNode.label}</h2>
              <div className="text-xs text-slate-400 mb-4">Node ID: {selectedNode.id}</div>

              <div className="space-y-3 text-xs">
                <div>
                  <div className="flex justify-between text-slate-400 mb-1">
                    <span>Recall Strength:</span>
                    <span className="text-emerald-400 font-bold">{(selectedNode.val * 100).toFixed(1)}%</span>
                  </div>
                  <div className="w-full bg-slate-800 h-1.5 rounded-full overflow-hidden">
                    <div
                      className="bg-gradient-to-r from-cyan-400 to-emerald-400 h-full"
                      style={{ width: `${selectedNode.val * 100}%` }}
                    />
                  </div>
                </div>

                <div className="bg-[#020408]/80 p-3 rounded-lg border border-slate-800/80 space-y-1">
                  <div className="text-slate-500 text-[10px] uppercase tracking-wider">Associated Artifacts</div>
                  <div className="text-slate-200 text-[11px] truncate">{selectedNode.files.join(', ') || 'None'}</div>
                </div>

                <div className="bg-[#020408]/80 p-3 rounded-lg border border-slate-800/80 space-y-1">
                  <div className="text-slate-500 text-[10px] uppercase tracking-wider">Narrative Context</div>
                  <div className="text-slate-300 text-[11px] leading-relaxed">{selectedNode.narrative}</div>
                </div>
              </div>
            </div>

            <div className="space-y-2 pt-4 border-t border-slate-800">
              <button
                onClick={() => {
                  sparksRef.current.forEach(s => { s.speed = 0.02 })
                  setTimeout(() => {
                    sparksRef.current.forEach(s => { s.speed = 0.003 + Math.random() * 0.004 })
                  }, 800)
                  playTone(980, 'sawtooth', 0.5, 0.22)
                }}
                className="w-full py-2 rounded-lg text-xs font-bold text-amber-400 border border-amber-500/40 bg-amber-950/20 hover:bg-amber-900/30 transition flex items-center justify-center gap-1.5"
              >
                <span>⚡ Pulse Action Potential</span>
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
