// Builds a 1024×1024 full-bleed app icon: an opaque background filling the whole square
// (macOS 26+ rounds the corners itself) with the artwork scaled up on top.
// A full-bleed icon avoids the grey/white tile macOS puts behind non-standard icon shapes.
// Usage: swift Scripts/compose-icon.swift <art.png> <out-1024.png> [art-scale 0–1, default 0.9]
import AppKit

let args = CommandLine.arguments
guard args.count >= 3 else { print("usage: compose-icon <art.png> <out.png> [scale]"); exit(1) }
let art = NSImage(contentsOfFile: args[1])!.cgImage(forProposedRect: nil, context: nil, hints: nil)!
let scale = args.count > 3 ? Double(args[3]) ?? 0.9 : 0.9

let size = 1024.0
let cs = CGColorSpace(name: CGColorSpace.sRGB)!
func color(_ r: Double, _ g: Double, _ b: Double, _ a: Double = 1) -> CGColor {
    CGColor(colorSpace: cs, components: [r / 255, g / 255, b / 255, a])!
}
let ctx = CGContext(data: nil, width: Int(size), height: Int(size), bitsPerComponent: 8, bytesPerRow: 0,
                    space: cs, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
ctx.interpolationQuality = .high

// Background: deep blue, lighter in the centre (echoes the blue inside the satchel's frame).
let center = CGPoint(x: size / 2, y: size / 2)
let bg = CGGradient(colorsSpace: cs, colors: [color(38, 74, 120), color(14, 26, 52)] as CFArray, locations: [0, 1])!
ctx.drawRadialGradient(bg, startCenter: center, startRadius: 0, endCenter: center, endRadius: size * 0.72,
                       options: [.drawsAfterEndLocation])
// Warm glow behind the artwork.
let glow = CGGradient(colorsSpace: cs, colors: [color(255, 190, 110, 0.28), color(255, 190, 110, 0)] as CFArray, locations: [0, 1])!
ctx.drawRadialGradient(glow, startCenter: center, startRadius: 0, endCenter: center, endRadius: size * 0.5, options: [])

// Artwork, large and centred, with a soft shadow.
let artSize = size * scale
ctx.setShadow(offset: CGSize(width: 0, height: -8), blur: 22, color: color(0, 0, 0, 0.5))
ctx.draw(art, in: CGRect(x: (size - artSize) / 2, y: (size - artSize) / 2, width: artSize, height: artSize))

let rep = NSBitmapImageRep(cgImage: ctx.makeImage()!)
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: args[2]))
print("wrote \(args[2])")
