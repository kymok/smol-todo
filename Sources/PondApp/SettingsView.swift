import Foundation
import SwiftUI
import TaskCore

private enum SettingsCategory: String, CaseIterable, Identifiable {
    case command
    case prompt
    case systemInformation

    var id: String { rawValue }

    var label: String {
        switch self {
        case .command:
            return "Command"
        case .prompt:
            return "Prompt"
        case .systemInformation:
            return "System Information"
        }
    }
}

struct SettingsView: View {
    @Environment(TaskAppModel.self) private var model
    @State private var defaultPromptTemplate = TaskPromptSettings.storedDefaultPromptTemplate
    @State private var selectedCategory: SettingsCategory = .command

    var body: some View {
        NavigationSplitView {
            List(SettingsCategory.allCases, selection: $selectedCategory) { category in
                Text(category.label)
                    .tag(category)
            }
            .navigationSplitViewColumnWidth(180)
            .toolbar(removing: .sidebarToggle)
        } detail: {
            Form {
                detailContent
            }
            .formStyle(.grouped)
        }
        .navigationSplitViewStyle(.balanced)
        .frame(width: 640, height: 420)
        .onAppear {
            model.refreshCLIStatus()
            defaultPromptTemplate = TaskPromptSettings.storedDefaultPromptTemplate
        }
        .onChange(of: defaultPromptTemplate) { _, promptTemplate in
            TaskPromptSettings.setDefaultPromptTemplate(promptTemplate)
        }
    }

    @ViewBuilder
    private var detailContent: some View {
        switch selectedCategory {
        case .command:
            commandSection
        case .prompt:
            promptSection
        case .systemInformation:
            systemInformationSection
        }
    }

    private var commandSection: some View {
        Section("Command Line") {
            if let status = model.cliStatus {
                LabeledContent("Link") {
                    Text(status.linkURL.path)
                        .textSelection(.enabled)
                }

                LabeledContent("Status") {
                    Text(statusText(status))
                        .foregroundStyle(status.installed ? .green : .secondary)
                }

                if !status.installDirectoryIsInPath {
                    LabeledContent("Add to PATH") {
                        HStack(spacing: 8) {
                            Text(model.pathHint)
                                .monospaced()
                                .textSelection(.enabled)

                            Button {
                                copyToPasteboard(model.pathHint)
                            } label: {
                                Image(systemName: "doc.on.doc")
                            }
                            .buttonStyle(.borderless)
                            .help("Copy PATH Command")
                        }
                    }
                }

                HStack {
                    Button("Reinstall") {
                        model.installCLI()
                    }
                    .disabled(!status.installed && !status.canInstall)

                    Button("Uninstall") {
                        model.uninstallCLI()
                    }
                    .disabled(!status.canUninstall)
                }
            }
        }
    }

    private var promptSection: some View {
        Section("Default App Prompt") {
            VStack(alignment: .leading, spacing: 8) {
                PromptTemplateEditor(
                    text: $defaultPromptTemplate,
                    height: 180
                )

                Button("Reset to Default") {
                    defaultPromptTemplate = TaskPromptTemplate.applicationDefaultTemplate.rawValue
                }
            }
        }
    }

    private var systemInformationSection: some View {
        Section("System Information") {
            LabeledContent("Version") {
                Text(appVersion)
                    .monospacedDigit()
                    .textSelection(.enabled)
            }

            LabeledContent("Build") {
                Text(buildNumber)
                    .monospacedDigit()
                    .textSelection(.enabled)
            }
        }
    }

    private var appVersion: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "Unavailable"
    }

    private var buildNumber: String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? "Unavailable"
    }

    private func statusText(_ status: CLIInstallStatus) -> String {
        if status.installed {
            return "Installed"
        }

        if let conflict = status.conflictDescription {
            return conflict
        }

        return "Not installed"
    }

}
