// ABOUTME: Wraps Sparkle's SPUStandardUpdaterController for the app's update menu.
// ABOUTME: Only activates when both SUFeedURL and SUPublicEDKey are configured (CI builds).

import Sparkle

@MainActor
final class SparkleUpdateController {
    private let updaterController: SPUStandardUpdaterController?

    init() {
        let feedURL = Bundle.main.object(forInfoDictionaryKey: "SUFeedURL") as? String ?? ""
        let publicKey = Bundle.main.object(forInfoDictionaryKey: "SUPublicEDKey") as? String ?? ""

        let hasFeed = !feedURL.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        let hasKey = !publicKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty

        if hasFeed && hasKey {
            updaterController = SPUStandardUpdaterController(
                startingUpdater: true,
                updaterDelegate: nil,
                userDriverDelegate: nil
            )
        } else {
            updaterController = nil
        }
    }

    var canCheckForUpdates: Bool {
        updaterController != nil
    }

    func checkForUpdates() {
        updaterController?.checkForUpdates(nil)
    }
}
