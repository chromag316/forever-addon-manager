// Sample data for previewing the UI in a plain browser (not loaded inside the app).
window.__mock = async (cmd, args) => {
  await new Promise((r) => setTimeout(r, 150));
  const s = window.__mockState ||= {
    installed: [
      { modId: 1697760, name: "EZFishing", author: "someone", version: "v1.1.2", kind: "curseforge", folders: ["EZFishing"], websiteUrl: "https://www.curseforge.com/wow/addons/ezfishing" },
      { modId: 1700375, name: "Spoken", author: "rustykey", version: "Spoken 3.10.1", kind: "curseforge", folders: ["Spoken"] },
      { modId: 1780747, name: "Spoken Audio: Bosses", author: "rustykey", version: "SpokenBossesAudio 1.0.1", kind: "curseforge", folders: ["SpokenBossesAudio"] },
      { modId: -618085760, name: "Forged Forever Auction House", version: "0.8.0", displayName: "0.8.0", kind: "git", sourceURL: "https://github.com/chromag316/Forged_ForeverAuctionHouse.git", folders: ["Forged_ForeverAuctionHouse"], websiteUrl: "https://github.com/chromag316/Forged_ForeverAuctionHouse" },
      { modId: -1043372642, name: "MyLocalAddon", version: "1.0", displayName: "1.0", kind: "folder", sourceURL: "/Users/me/Dev/MyLocalAddon", folders: ["MyLocalAddon"] },
    ],
    untracked: ["Blizzard_Something"],
    settings: { addonsPath: "/Applications/World of Warcraft/_forever_/Interface/AddOns", gameVersionTypeId: 88568, releaseType: 1 },
  };
  switch (cmd) {
    case "overview": return { ...s, autoUpdates: true, hasKey: true, addonsPathExists: true, appVersion: "0.1.0" };
    case "check_updates": return [{ modId: 1700375, name: "Spoken", versionLabel: "Spoken 3.11.0", websiteOnly: false },
                                  { modId: 1697760, name: "EZFishing", versionLabel: "v1.1.3", websiteOnly: true }];
    case "browse": {
      // 10 pages of 20 sample addons; set window.__mockFail = true to simulate a network error.
      if (window.__mockFail) throw "Couldn't connect to CurseForge. Check your internet connection.";
      const start = args.excluding.length;
      if (start >= 200) return [];
      return Array.from({ length: 20 }, (_, i) => ({ id: 5000 + start + i, name: `Sample Addon ${start + i + 1}`,
        summary: "A sample addon used for previewing the list.", downloadCount: 1000 * (start + i + 1) }));
    }
    case "update_all": return { updated: ["Spoken"], failed: [] };
    case "version_types": return [{ id: 88568, name: "WoW Forever" }];
    case "test_key": return "Key length: 60 characters.\n✅ CurseForge accepted the key.";
    case "set_key": return null;
    default: return null;
  }
};
