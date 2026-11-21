// Sample data for previewing the UI in a plain browser (not loaded inside the app).
// Open with ?screenshot for the README screenshot: real public addons, one pending update.
const SCREENSHOT = location.search.includes("screenshot");
const cdnThumb = (p) => `https://media.forgecdn.net/avatars/thumbnails/${p}`;
const SCREENSHOT_STATE = {
  installed: [
    { modId: 1697760, name: "EZFishing", author: "BuggedOnPurpose", version: "v1.1.2", kind: "curseforge", folders: ["EZFishing"], thumbnailUrl: cdnThumb("2049/843/256/256/639251134413919860.jpg") },
    { modId: 1700375, name: "Spoken", author: "rustykey", version: "Spoken 3.11.0", kind: "curseforge", folders: ["Spoken"], thumbnailUrl: cdnThumb("2123/272/256/256/639277523184843230.png") },
    { modId: 1780747, name: "Spoken Audio: Bosses", author: "rustykey", version: "SpokenBossesAudio 1.0.1", kind: "curseforge", folders: ["SpokenBossesAudio"], thumbnailUrl: cdnThumb("2218/578/256/256/639305969734520374.png") },
    { modId: 1660202, name: "Spoken Audio: Gossip", author: "rustykey", version: "SpokenQuestsAudioGossip 2.7.1", kind: "curseforge", folders: ["SpokenQuestsAudioGossip"], thumbnailUrl: cdnThumb("2123/271/256/256/639277523166345709.png") },
    { modId: 1660197, name: "Spoken Audio: Quests - Alliance", author: "rustykey", version: "SpokenQuestsAudioAlliance 2.7.1", kind: "curseforge", folders: ["SpokenQuestsAudioAlliance"], thumbnailUrl: cdnThumb("2123/267/256/256/639277523062039367.png") },
    { modId: 1660199, name: "Spoken Audio: Quests - Shared", author: "rustykey", version: "SpokenQuestsAudioShared 2.7.1", kind: "curseforge", folders: ["SpokenQuestsAudioShared"], thumbnailUrl: cdnThumb("2123/269/256/256/639277523148029412.png") },
    { modId: 1701520, name: "Spoken Audio: Readables", author: "rustykey", version: "SpokenReadablesAudio 2.3.1", kind: "curseforge", folders: ["SpokenReadablesAudio"], thumbnailUrl: cdnThumb("2123/277/256/256/639277524393000183.png") },
    { modId: -618085760, name: "Forged Forever Auction House", version: "0.8.0", displayName: "0.8.0", kind: "git", sourceURL: "https://github.com/chromag316/Forged_ForeverAuctionHouse.git", folders: ["Forged_ForeverAuctionHouse"] },
    { modId: -1804290343, name: "Forged Forever Map Notes", version: "1.0.2", displayName: "1.0.2", kind: "git", sourceURL: "https://github.com/chromag316/Forged_ForeverMapNotes.git", folders: ["Forged_ForeverMapNotes"] },
  ],
  untracked: [],
  settings: { addonsPath: "/Applications/World of Warcraft/_forever_/Interface/AddOns", gameVersionTypeId: 88568, releaseType: 1 },
};

// Real popular WoW Forever addons from CurseForge (public data), for the Get Addons screenshot.
const SCREENSHOT_CATALOG = [
  {
    "id": 334372,
    "name": "Questie",
    "summary": "The Classic Quest Helper",
    "downloadCount": 228476448,
    "allowModDistribution": true,
    "logo": {
      "thumbnailUrl": "https://media.forgecdn.net/avatars/thumbnails/351/162/256/256/637505206198965674.png"
    }
  },
  {
    "id": 61284,
    "name": "Details! Damage Meter",
    "summary": "Complete combat analysis, gathering damage, healing, and other importante stuff.",
    "downloadCount": 376869212,
    "allowModDistribution": false,
    "logo": {
      "thumbnailUrl": "https://media.forgecdn.net/avatars/thumbnails/109/996/256/256/636383851490858804.png"
    }
  },
  {
    "id": 3358,
    "name": "DBM - Deadly Boss Mods (DBM-Core)",
    "summary": "A Swiss Army knife that gives you fight info & tools for dungeons & raids. Supporting Midnight, Forever, Mists of Pandaria, Wrath Titan, Burning Crusade, Vanilla.",
    "downloadCount": 643773173,
    "allowModDistribution": false,
    "logo": {
      "thumbnailUrl": "https://media.forgecdn.net/avatars/thumbnails/199/661/256/256/636908483866103322.png"
    }
  },
  {
    "id": 6124,
    "name": "Auctionator",
    "summary": "Makes it easier to use the auction house and track the value of items",
    "downloadCount": 215914951,
    "allowModDistribution": false,
    "logo": {
      "thumbnailUrl": "https://media.forgecdn.net/avatars/thumbnails/1403/232/256/256/638909541952729618.png"
    }
  },
  {
    "id": 84208,
    "name": "RareScanner",
    "summary": "RareScanner, a World of Warcraft addon to alert you when a rare, event or treasure appear in your minimap.",
    "downloadCount": 147470080,
    "allowModDistribution": true,
    "logo": {
      "thumbnailUrl": "https://media.forgecdn.net/avatars/thumbnails/58/841/256/256/636141449063964125.PNG"
    }
  },
  {
    "id": 1592,
    "name": "Bagnon",
    "summary": "See your inventory, bank, guild and alts from anywhere",
    "downloadCount": 172952144,
    "allowModDistribution": false,
    "logo": {
      "thumbnailUrl": "https://media.forgecdn.net/avatars/thumbnails/647/705/256/256/638048338012580367.png"
    }
  },
  {
    "id": 94855,
    "name": "Leatrix Plus (USE THIS ONE)",
    "summary": "Powerful and modular quality of life addon with a small resource footprint. Supports all WoW games including Midnight and Burning Crusade Classic Anniversary (as well as all the others).",
    "downloadCount": 79629868,
    "allowModDistribution": false,
    "logo": {
      "thumbnailUrl": "https://media.forgecdn.net/avatars/thumbnails/483/32/256/256/637782408501667202.png"
    }
  },
  {
    "id": 1710300,
    "name": "Delete Cheapest Item: Updated for Forever",
    "summary": "Delete the least valuable junk item in your bags when looting",
    "downloadCount": 283951,
    "allowModDistribution": true,
    "logo": {
      "thumbnailUrl": "https://media.forgecdn.net/avatars/thumbnails/2068/55/256/256/639258740258861510.png"
    }
  },
  {
    "id": 1718307,
    "name": "FlareUI",
    "summary": "Your friendly UI overhowl for WoW: Forever. Improving Blizzard's UI look and feel without replacing it.",
    "downloadCount": 223119,
    "allowModDistribution": true,
    "logo": {
      "thumbnailUrl": "https://media.forgecdn.net/avatars/thumbnails/2106/737/256/256/639272312602401423.png"
    }
  },
  {
    "id": 1707971,
    "name": "DungeonJournal",
    "summary": "Check which quests are available for each dungeons, where they are , and what loot bosses drop",
    "downloadCount": 19321168,
    "allowModDistribution": true,
    "logo": {
      "thumbnailUrl": "https://media.forgecdn.net/avatars/thumbnails/2218/764/256/256/639306029966247610.png"
    }
  }
];

// ?screenshot=browse opens the Get Addons tab.
if (location.search.includes("screenshot=browse")) {
  window.addEventListener("load", () => setTimeout(() => document.querySelector('[data-tab="browse"]').click(), 50));
}

window.__mock = async (cmd, args) => {
  await new Promise((r) => setTimeout(r, SCREENSHOT ? 0 : 150));
  if (SCREENSHOT) {
    if (cmd === "overview") return { ...SCREENSHOT_STATE, autoUpdates: false, hasKey: true, addonsPathExists: true, appVersion: "1.0.0" };
    if (cmd === "check_updates") return [{ modId: 1660202, name: "Spoken Audio: Gossip", versionLabel: "SpokenQuestsAudioGossip 2.7.2", websiteOnly: false }];
    if (cmd === "browse") return args.excluding.length ? [] : SCREENSHOT_CATALOG;
  }
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
