// Renders a 1024×1024 full-bleed app icon: a charcoal tile (Terminal-style gradient) with a
// checklist of three addons (puzzle piece, text lines, green check).
// macOS rounds the corners itself.
// Usage: swift Scripts/make-list-icon.swift <out.png>
import AppKit

let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "list-icon.png"
let S = 1024.0
let cs = CGColorSpace(name: CGColorSpace.sRGB)!
func c(_ hex: UInt32, _ a: Double = 1) -> CGColor {
    CGColor(colorSpace: cs, components: [Double((hex >> 16) & 0xff) / 255, Double((hex >> 8) & 0xff) / 255,
                                         Double(hex & 0xff) / 255, a])!
}
let ctx = CGContext(data: nil, width: Int(S), height: Int(S), bitsPerComponent: 8, bytesPerRow: 0,
                    space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!

// Background: charcoal, lighter at the top to near-black at the bottom (like Terminal's icon),
// with a faint highlight along the top edge.
let bg = CGGradient(colorsSpace: cs, colors: [c(0x3D3D40), c(0x262628), c(0x151517)] as CFArray, locations: [0, 0.5, 1])!
ctx.drawLinearGradient(bg, start: CGPoint(x: 0, y: S), end: CGPoint(x: 0, y: 0), options: [])
let sheen = CGGradient(colorsSpace: cs, colors: [c(0xFFFFFF, 0.10), c(0xFFFFFF, 0)] as CFArray, locations: [0, 1])!
ctx.drawLinearGradient(sheen, start: CGPoint(x: 0, y: S), end: CGPoint(x: 0, y: S - 70), options: [])

func fillGradient(_ shape: CGPath, _ g: CGGradient, top: Double, bottom: Double) {
    ctx.saveGState()
    ctx.addPath(shape); ctx.clip()
    ctx.drawLinearGradient(g, start: CGPoint(x: 0, y: top), end: CGPoint(x: 0, y: bottom),
                           options: [.drawsBeforeStartLocation, .drawsAfterEndLocation])
    ctx.restoreGState()
}
/// Metallic shape: dark outline + shadow, then gradient body.
func drawMetal(_ shape: CGPath, _ g: CGGradient, top: Double, bottom: Double, outline: Double = 9,
               edge: UInt32 = 0x4A2A06, shadow: Bool = true) {
    ctx.saveGState()
    if shadow { ctx.setShadow(offset: CGSize(width: 0, height: -8), blur: 18, color: c(0x000000, 0.6)) }
    ctx.addPath(shape.copy(strokingWithWidth: outline, lineCap: .round, lineJoin: .round, miterLimit: 10))
    ctx.addPath(shape)
    ctx.setFillColor(c(edge)); ctx.fillPath(using: .winding)
    ctx.restoreGState()
    fillGradient(shape, g, top: top, bottom: bottom)
}

// Jigsaw piece path (knobs top/right, sockets left/bottom)
func jigsaw(center: CGPoint, body b: Double) -> CGPath {
    let r = b * 0.17, k = b * 0.10
    let x0 = center.x - b / 2, y0 = center.y - b / 2, x1 = x0 + b, y1 = y0 + b
    let cx = center.x, cy = center.y, neck = asin(min(1, k / r))
    let p = CGMutablePath()
    p.move(to: CGPoint(x: x0, y: y0))
    p.addLine(to: CGPoint(x: cx - r * cos(neck), y: y0))
    p.addArc(center: CGPoint(x: cx, y: y0 + k), radius: r, startAngle: .pi + neck, endAngle: -neck, clockwise: true)
    p.addLine(to: CGPoint(x: x1, y: y0))
    p.addLine(to: CGPoint(x: x1, y: cy - r * cos(neck)))
    p.addArc(center: CGPoint(x: x1 + k, y: cy), radius: r, startAngle: -.pi / 2 - neck, endAngle: .pi / 2 + neck, clockwise: false)
    p.addLine(to: CGPoint(x: x1, y: y1))
    p.addLine(to: CGPoint(x: cx + r * cos(neck), y: y1))
    p.addArc(center: CGPoint(x: cx, y: y1 + k), radius: r, startAngle: -neck, endAngle: .pi + neck, clockwise: false)
    p.addLine(to: CGPoint(x: x0, y: y1))
    p.addLine(to: CGPoint(x: x0, y: cy + r * cos(neck)))
    p.addArc(center: CGPoint(x: x0 + k, y: cy), radius: r, startAngle: .pi / 2 + neck, endAngle: -.pi / 2 - neck, clockwise: true)
    p.closeSubpath()
    return p
}

let steel = CGGradient(colorsSpace: cs, colors: [c(0xDCEBFA), c(0x9DB6D3), c(0x6F86A3)] as CFArray, locations: [0, 0.55, 1])!
let green = CGGradient(colorsSpace: cs, colors: [c(0x9CFF6A), c(0x3CC71F), c(0x248F12)] as CFArray, locations: [0, 0.6, 1])!

// Three rows: piece, long + short text line, check. Drawn in row coordinates, then centred on
// the tile and scaled up to use the space (content spans x 164–873, y 186–774 unscaled).
let contentCenter = CGPoint(x: 518.5, y: 480)
let scale = 1.1
ctx.translateBy(x: S / 2, y: S / 2)
ctx.scaleBy(x: scale, y: scale)
ctx.translateBy(x: -contentCenter.x, y: -contentCenter.y)
for (i, rowY) in [668.0, 468.0, 268.0].enumerated() {
    let pc = CGPoint(x: 238, y: rowY - 8)
    let piece = jigsaw(center: pc, body: 148)
    drawMetal(piece, steel, top: pc.y + 100, bottom: pc.y - 74, outline: 8, edge: 0x2E3A4A)

    let long = CGPath(roundedRect: CGRect(x: 382, y: rowY + 6, width: 248, height: 38), cornerWidth: 18, cornerHeight: 18, transform: nil)
    let short = CGPath(roundedRect: CGRect(x: 382, y: rowY - 46, width: i == 0 ? 160 : 130, height: 28), cornerWidth: 13, cornerHeight: 13, transform: nil)
    ctx.addPath(long); ctx.setFillColor(c(0x6A6D76)); ctx.fillPath()
    ctx.addPath(short); ctx.setFillColor(c(0x4C4F57)); ctx.fillPath()

    let m = CGMutablePath()
    m.move(to: CGPoint(x: 680, y: rowY + 8))
    m.addLine(to: CGPoint(x: 735, y: rowY - 50))
    m.addLine(to: CGPoint(x: 850, y: rowY + 70))
    drawMetal(m.copy(strokingWithWidth: 46, lineCap: .round, lineJoin: .round, miterLimit: 10), green,
              top: rowY + 70, bottom: rowY - 50, outline: 8, edge: 0x14420A)
}

try! NSBitmapImageRep(cgImage: ctx.makeImage()!).representation(using: .png, properties: [:])!
    .write(to: URL(fileURLWithPath: out))
print("wrote \(out)")
