// Renders a 1024×1024 full-bleed app icon: one big jigsaw piece with a sideways
// infinity loop on it. macOS rounds the corners itself.
// Usage: swift Scripts/make-puzzle-icon.swift <out.png> [variant: lightblue | gold | blue | light]
import AppKit

let args = CommandLine.arguments
let out = args.count > 1 ? args[1] : "puzzle-icon.png"
let variant = args.count > 2 ? args[2] : "lightblue"

let S = 1024.0
let cs = CGColorSpace(name: CGColorSpace.sRGB)!
func c(_ hex: UInt32, _ a: Double = 1) -> CGColor {
    CGColor(colorSpace: cs, components: [Double((hex >> 16) & 0xff) / 255, Double((hex >> 8) & 0xff) / 255,
                                         Double(hex & 0xff) / 255, a])!
}

struct Palette { let bgTop, bgBottom, pieceTop, pieceBottom, pieceEdge, symbol: UInt32; let symbolGlow: Bool }
let palettes: [String: Palette] = [
    // Gold piece on midnight blue, engraved dark-blue infinity
    "gold":  Palette(bgTop: 0x2B4C8C, bgBottom: 0x0E1A3A, pieceTop: 0xFFD877, pieceBottom: 0xE09A2B,
                     pieceEdge: 0x9A5E12, symbol: 0x13244D, symbolGlow: false),
    // Bright blue piece on near-black, glowing gold infinity
    "blue":  Palette(bgTop: 0x23262E, bgBottom: 0x0B0C10, pieceTop: 0x5FB8FF, pieceBottom: 0x1F6FE0,
                     pieceEdge: 0x124B9E, symbol: 0xFFD45C, symbolGlow: true),
    // White piece on blue, blue infinity (the app icon)
    "lightblue": Palette(bgTop: 0x5AA2FF, bgBottom: 0x1D4FC4, pieceTop: 0xFFFFFF, pieceBottom: 0xE3ECFA,
                         pieceEdge: 0xA9BEDF, symbol: 0x2563EB, symbolGlow: false),
    // White piece on violet, violet infinity
    "light": Palette(bgTop: 0x8A5CF6, bgBottom: 0x4A1FA8, pieceTop: 0xFFFFFF, pieceBottom: 0xE6E1F5,
                     pieceEdge: 0xB9AEDB, symbol: 0x6B3FD8, symbolGlow: false),
]
guard let p = palettes[variant] else { print("unknown variant \(variant)"); exit(1) }

let ctx = CGContext(data: nil, width: Int(S), height: Int(S), bitsPerComponent: 8, bytesPerRow: 0,
                    space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!

// Background (full bleed)
let bg = CGGradient(colorsSpace: cs, colors: [c(p.bgTop), c(p.bgBottom)] as CFArray, locations: [0, 1])!
ctx.drawLinearGradient(bg, start: CGPoint(x: 0, y: S), end: CGPoint(x: 0, y: 0), options: [])

// Jigsaw piece: rounded square body, knobs on left and right, sockets on top and bottom,
// so the piece is symmetric and the infinity loop has room in the exact centre.
let body = 560.0, r = 92.0, knobOffset = 52.0   // knob circle centre sits this far outside the edge
let ox = (S - body) / 2, oy = (S - body) / 2
let bodyRect = CGRect(x: ox, y: oy, width: body, height: body)

func piecePath() -> CGPath {
    // Build with a temporary mask: body + knobs − sockets, traced via even-odd composition.
    let p = CGMutablePath()
    p.addPath(CGPath(roundedRect: bodyRect, cornerWidth: 46, cornerHeight: 46, transform: nil))
    return p
}

// Draw the piece shape into an alpha mask so knobs/sockets combine cleanly.
let mask = CGContext(data: nil, width: Int(S), height: Int(S), bitsPerComponent: 8, bytesPerRow: 0,
                     space: CGColorSpaceCreateDeviceGray(), bitmapInfo: CGImageAlphaInfo.none.rawValue)!
mask.setFillColor(gray: 0, alpha: 1); mask.fill(CGRect(x: 0, y: 0, width: S, height: S))
mask.setFillColor(gray: 1, alpha: 1)
mask.addPath(piecePath()); mask.fillPath()
let midX = bodyRect.midX, midY = bodyRect.midY
// knobs (left, right), with necks filled so they join smoothly
mask.fillEllipse(in: CGRect(x: bodyRect.minX - knobOffset - r, y: midY - r, width: 2 * r, height: 2 * r))
mask.fillEllipse(in: CGRect(x: bodyRect.maxX + knobOffset - r, y: midY - r, width: 2 * r, height: 2 * r))
mask.fill(CGRect(x: bodyRect.minX - knobOffset, y: midY - r * 0.62, width: knobOffset + 10, height: r * 1.24))
mask.fill(CGRect(x: bodyRect.maxX - 10, y: midY - r * 0.62, width: knobOffset + 10, height: r * 1.24))
// sockets (top, bottom)
mask.setFillColor(gray: 0, alpha: 1)
mask.fillEllipse(in: CGRect(x: midX - r, y: bodyRect.maxY - knobOffset - r, width: 2 * r, height: 2 * r))
mask.fillEllipse(in: CGRect(x: midX - r, y: bodyRect.minY + knobOffset - r, width: 2 * r, height: 2 * r))
mask.fill(CGRect(x: midX - r * 0.62, y: bodyRect.maxY - knobOffset, width: r * 1.24, height: knobOffset + 10))
mask.fill(CGRect(x: midX - r * 0.62, y: bodyRect.minY - 10, width: r * 1.24, height: knobOffset + 10))
let pieceMask = mask.makeImage()!

// Shadow under the piece
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: -18), blur: 40, color: c(0x000000, 0.45))
ctx.beginTransparencyLayer(auxiliaryInfo: nil)
ctx.clip(to: CGRect(x: 0, y: 0, width: S, height: S), mask: pieceMask)
ctx.setFillColor(c(p.pieceEdge)); ctx.fill(CGRect(x: 0, y: 0, width: S, height: S))
ctx.endTransparencyLayer()
ctx.restoreGState()

// Piece face: gradient, inset slightly from the darker edge to give a bevel.
ctx.saveGState()
ctx.clip(to: CGRect(x: 0, y: -10, width: S, height: S), mask: pieceMask)   // shifted up = darker rim at bottom
let face = CGGradient(colorsSpace: cs, colors: [c(p.pieceTop), c(p.pieceBottom)] as CFArray, locations: [0, 1])!
ctx.drawLinearGradient(face, start: CGPoint(x: 0, y: bodyRect.maxY + 150), end: CGPoint(x: 0, y: bodyRect.minY),
                       options: [.drawsBeforeStartLocation, .drawsAfterEndLocation])
ctx.restoreGState()

// Infinity loop (lemniscate of Bernoulli) centred on the piece body.
func lemniscate(center: CGPoint, a: Double) -> CGPath {
    let path = CGMutablePath()
    for i in 0...720 {
        let t = Double(i) / 720 * 2 * .pi
        let d = 1 + sin(t) * sin(t)
        let pt = CGPoint(x: center.x + a * cos(t) / d, y: center.y + a * sin(t) * cos(t) / d * 1.25)
        i == 0 ? path.move(to: pt) : path.addLine(to: pt)
    }
    path.closeSubpath()
    return path
}
// Centred on the piece, between the top and bottom sockets.
let loop = lemniscate(center: CGPoint(x: midX, y: midY), a: 196)
let stroke = loop.copy(strokingWithWidth: 56, lineCap: .round, lineJoin: .round, miterLimit: 10)
ctx.saveGState()
if p.symbolGlow {
    ctx.setShadow(offset: .zero, blur: 26, color: c(p.symbol, 0.8))
} else {
    // engraved look: light highlight below, symbol on top
    ctx.saveGState()
    ctx.translateBy(x: 0, y: -4)
    ctx.addPath(stroke); ctx.setFillColor(c(0xFFFFFF, 0.35)); ctx.fillPath()
    ctx.restoreGState()
}
ctx.addPath(stroke)
ctx.setFillColor(c(p.symbol))
ctx.fillPath()
ctx.restoreGState()

try! NSBitmapImageRep(cgImage: ctx.makeImage()!).representation(using: .png, properties: [:])!
    .write(to: URL(fileURLWithPath: out))
print("wrote \(out)")
