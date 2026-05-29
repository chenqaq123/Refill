import SwiftUI

enum AppDesign {
    static let pageMaxWidth: CGFloat = 880
    static let cardRadius: CGFloat = 12
    static let buttonRadius: CGFloat = 9

    static let blue = Color(red: 0.12, green: 0.44, blue: 0.86)
    static let teal = Color(red: 0.08, green: 0.50, blue: 0.48)
    static let green = Color(red: 0.12, green: 0.62, blue: 0.30)
    static let orange = Color(red: 0.82, green: 0.48, blue: 0.16)
    static let red = Color(red: 0.76, green: 0.20, blue: 0.22)

    static var appBackground: LinearGradient {
        LinearGradient(
            colors: [
                Color(red: 0.982, green: 0.984, blue: 0.986),
                Color(red: 0.972, green: 0.976, blue: 0.980),
                Color(red: 0.992, green: 0.992, blue: 0.988)
            ],
            startPoint: .topLeading,
            endPoint: .bottomTrailing
        )
    }

    static var cardFill: some ShapeStyle {
        .regularMaterial
    }
}

struct ToolbarActionButton: View {
    let title: String
    let systemImage: String
    var tint: Color = AppDesign.blue
    let action: () -> Void

    @State private var isHovering = false

    var body: some View {
        Button(action: action) {
            Label(title, systemImage: systemImage)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(isHovering ? .white : tint)
                .padding(.horizontal, 11)
                .padding(.vertical, 7)
                .background {
                    RoundedRectangle(cornerRadius: AppDesign.buttonRadius, style: .continuous)
                        .fill(isHovering ? tint : tint.opacity(0.10))
                }
                .overlay {
                    RoundedRectangle(cornerRadius: AppDesign.buttonRadius, style: .continuous)
                        .stroke(tint.opacity(isHovering ? 0 : 0.20), lineWidth: 1)
                }
        }
        .buttonStyle(.plain)
        .scaleEffect(isHovering ? 1.025 : 1)
        .animation(.easeOut(duration: 0.15), value: isHovering)
        .onHover { isHovering = $0 }
    }
}

struct StatusChip: View {
    let title: String
    let systemImage: String
    var tint: Color = AppDesign.blue
    var filled = false

    var body: some View {
        Label(title, systemImage: systemImage)
            .font(.system(size: 11, weight: .semibold))
            .foregroundStyle(filled ? .white : tint)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background {
                Capsule(style: .continuous)
                    .fill(filled ? tint : tint.opacity(0.12))
            }
    }
}

struct CardSurface: ViewModifier {
    let isActive: Bool
    let isHovering: Bool

    func body(content: Content) -> some View {
        content
            .background {
                RoundedRectangle(cornerRadius: AppDesign.cardRadius, style: .continuous)
                    .fill(AppDesign.cardFill)
                    .shadow(
                        color: .black.opacity(isHovering ? 0.10 : 0.045),
                        radius: isHovering ? 16 : 8,
                        x: 0,
                        y: isHovering ? 7 : 3
                    )
            }
            .overlay {
                RoundedRectangle(cornerRadius: AppDesign.cardRadius, style: .continuous)
                    .stroke(
                        isActive ? AppDesign.blue.opacity(0.55) : Color.primary.opacity(isHovering ? 0.14 : 0.08),
                        lineWidth: isActive ? 1.4 : 1
                    )
            }
    }
}

extension View {
    func cardSurface(isActive: Bool = false, isHovering: Bool = false) -> some View {
        modifier(CardSurface(isActive: isActive, isHovering: isHovering))
    }
}
