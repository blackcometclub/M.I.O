// Run with Node 24+. MIO_PLAYWRIGHT_MODULE may point to a bundled playwright entry.
// Uses an isolated headless browser and mocked desktop bridges; no provider calls.
import assert from "node:assert/strict";
import { fileURLToPath, pathToFileURL } from "node:url";
import { createServer } from "vite";

const { chromium } = await import(process.env.MIO_PLAYWRIGHT_MODULE
  ? pathToFileURL(process.env.MIO_PLAYWRIGHT_MODULE).href : "playwright");
const mocks = {
  "../roomBridge": `
    export * from '/src/roomBridge.ts';
    export async function readDesktopRooms() {
      await window.catalogReady;
      if (window.catalogFails) throw new Error('catalog unavailable');
      return structuredClone(window.fixture);
    }
    export async function readDesktopAiConnectionStatuses() { return {}; }
    export async function readDesktopRoomDispatchUnknowns() { return []; }
    export async function readDesktopRoomWorkspaceStatus(roomId) {
      return {roomId, mode:'chatOnly', folderName:null, available:true};
    }
    export async function readDesktopRoomConductorStatus(roomId) {
      return {roomId, conductorId:null, sendMode:'direct'};
    }
    export async function readDesktopRoomBackupStatus() {
      return {directoryPath:'fixture', isCustom:false, available:true};
    }
    export async function deleteDesktopRoom({roomId}) {
      window.fixture.rooms = window.fixture.rooms.filter(room => room.id !== roomId);
    }
    export async function createDesktopRoom({roomId, name}) {
      const room = {id:roomId,name,participantIds:['owner','codex'],messages:[],updatedLabel:''};
      window.fixture.rooms.push(room);
      return room;
    }
    export async function previewLatestDesktopRoomBackup() { return {fileName:'fixture.json'}; }
    export async function restoreDesktopRoomBackup() {
      window.fixture.rooms = window.fixture.rooms.filter(room => window.restoreIds.includes(room.id));
      return {fileName:'fixture.json',roomCount:window.fixture.rooms.length};
    }
  `,
  "../commandConfirmationBridge": `
    export * from '/src/commandConfirmationBridge.ts';
    export async function activateDesktopCommandRoomSession() {}
    export async function deactivateDesktopCommandRoomSession() {}
  `,
  "../participantProfileBridge": `
    export * from '/src/participantProfileBridge.ts';
    export async function readParticipantProfiles() { return []; }
  `,
  "@tauri-apps/api/event": `export async function listen() { return () => {}; }`,
};
const html = `<!doctype html><link rel="icon" href="data:,"><div id="root"></div><script type="module">
  import React, {useEffect} from 'react';
  import {createRoot} from 'react-dom/client';
  import {useRooms} from '/src/hooks/useRooms.ts';
  import {UiPreferencesProvider} from '/src/uiPreferences.tsx';
  import {initialRooms,demoParticipants} from '/src/mockData.ts';
  window.fixture = {participants:demoParticipants,rooms:[initialRooms[0],
    {...initialRooms[0],id:'room-a',name:'A',participantIds:['owner','gemini'],messages:[]},
    {...initialRooms[0],id:'room-b',name:'B',participantIds:['owner','codex'],messages:[]}]};
  function Fixture() {
    const state = useRooms();
    useEffect(() => { window.roomTest = state; });
    return React.createElement('output',null,state.roomSourceMode+':'+state.activeRoom.id);
  }
  createRoot(document.getElementById('root')).render(
    React.createElement(React.StrictMode,null,
      React.createElement(UiPreferencesProvider,null,React.createElement(Fixture))));
</script>`;
const server = await createServer({
  configFile: false,
  root: fileURLToPath(new URL("../apps/desktop", import.meta.url)),
  server: { host: "127.0.0.1", port: 0 },
  plugins: [{
    name: "room-selection-fixture",
    enforce: "pre",
    resolveId(source, importer) {
      if (source in mocks && importer?.endsWith("/hooks/useRooms.ts")) return `\0fixture:${source}`;
    },
    load(id) { if (id.startsWith("\0fixture:")) return mocks[id.slice(9)]; },
    configureServer(vite) {
      vite.middlewares.use("/room-selection-test.html", async (_req, res) => {
        res.setHeader("Content-Type", "text/html");
        res.end(await vite.transformIndexHtml('/room-selection-test.html', html));
      });
    },
  }],
});
let browser;
let passed = 0;
const backendKey = "moe-active-room-v1-backend";
try {
  await server.listen();
  const url = `${server.resolvedUrls.local[0]}room-selection-test.html`;
  browser = await chromium.launch({ channel: "msedge", headless: true });
  async function scenario(name, options, run) {
    const context = await browser.newContext();
    try {
      await context.addInitScript(({saved, demo = false, storageFails = false, writesFail = false, catalogFails = false}) => {
        if (!sessionStorage.getItem('fixture-seeded')) {
          if (saved !== undefined) localStorage.setItem('moe-active-room-v1-backend', saved);
          localStorage.setItem('moe-active-room-v1-browserDemo', 'comparison-room');
          sessionStorage.setItem('fixture-seeded', 'yes');
        }
        window.catalogFails = catalogFails;
        window.catalogReady = new Promise(resolve => { window.releaseCatalog = resolve; });
        if (!demo) window.__TAURI_INTERNALS__ = {invoke: async () => { throw new Error('Unmocked native call'); }};
        if (storageFails) {
          Object.defineProperty(window, 'localStorage', {get() { throw new Error('storage denied'); }});
        }
        if (writesFail) Storage.prototype.setItem = () => { throw new Error('quota exceeded'); };
      }, options);
      const page = await context.newPage();
      page.setDefaultTimeout(15_000);
      const errors = [];
      page.on("console", message => { if (message.type() === "error") console.error(message.text()); });
      page.on("pageerror", error => { errors.push(error.message); console.error(error.message); });
      await page.goto(url);
      await page.waitForFunction(() => !!window.roomTest);
      await run(page);
      assert.deepEqual(errors, []);
      console.log(`PASS ${name}`);
      passed++;
    } finally { await context.close(); }
  }
  async function hydrate(page, expected) {
    await page.evaluate(() => window.releaseCatalog());
    await page.waitForFunction(id => window.roomTest.roomSourceMode === 'backend'
      && window.roomTest.activeRoom.id === id, expected);
  }
  async function stored(page, expected) {
    await page.waitForFunction(({key,id}) => localStorage.getItem(key) === id, {key:backendKey,id:expected});
  }
  await scenario("delayed hydration preserves selection and restores recipients", {saved:"room-a"}, async page => {
    assert.equal(await page.evaluate(key => localStorage.getItem(key), backendKey), "room-a");
    await hydrate(page, "room-a");
    assert.deepEqual(await page.evaluate(() => window.roomTest.recipientIds), ["gemini"]);
    await page.evaluate(() => window.roomTest.selectRoom('room-b'));
    await stored(page, "room-b");
    await page.reload();
    await page.waitForFunction(() => !!window.roomTest);
    await hydrate(page, "room-b");
    await stored(page, "room-b");
  });
  for (const [name, saved] of [["first launch",undefined],["deleted ID","missing-room"],
    ["malformed ID","bad\nroom"],["empty ID",""],["oversized ID","x".repeat(257)]]) {
    await scenario(name, {saved}, async page => { await hydrate(page,"moe-dev-room"); await stored(page,"moe-dev-room"); });
  }
  await scenario("unavailable storage keeps navigation working", {storageFails:true}, async page => {
    await hydrate(page,"moe-dev-room");
    await page.evaluate(() => window.roomTest.selectRoom('room-a'));
    await page.waitForFunction(() => window.roomTest.activeRoom.id === 'room-a');
  });
  await scenario("catalog failure does not overwrite saved selection", {saved:"room-a",catalogFails:true}, async page => {
    await page.evaluate(() => window.releaseCatalog());
    await page.waitForFunction(() => window.roomTest.roomSourceMode === 'error');
    assert.equal(await page.evaluate(key => localStorage.getItem(key), backendKey), "room-a");
  });
  await scenario("storage write failure does not prevent navigation", {saved:"room-a",writesFail:true}, async page => {
    await hydrate(page,"room-a");
    await page.evaluate(() => window.roomTest.selectRoom('room-b'));
    await page.waitForFunction(() => window.roomTest.activeRoom.id === 'room-b');
    assert.equal(await page.evaluate(key => localStorage.getItem(key), backendKey),"room-a");
  });
  await scenario("browser demo does not overwrite desktop preference", {saved:"room-a",demo:true}, async page => {
    await page.waitForFunction(() => window.roomTest.roomSourceMode === 'browserDemo');
    assert.equal(await page.evaluate(() => window.roomTest.activeRoom.id), "comparison-room");
    assert.equal(await page.evaluate(key => localStorage.getItem(key), backendKey), "room-a");
  });
  await scenario("deleting the selected Room remembers the fallback", {saved:"room-a"}, async page => {
    await hydrate(page,"room-a");
    assert.equal(await page.evaluate(() => window.roomTest.deleteRoom()), true);
    await stored(page,"moe-dev-room");
  });
  for (const keepSelection of [true,false]) {
    await scenario(`backup restore ${keepSelection ? 'retains current Room' : 'falls back if absent'}`, {saved:"room-a"}, async page => {
      await hydrate(page,"room-a");
      await page.evaluate(keep => { window.restoreIds = keep ? ['moe-dev-room','room-a'] : ['room-b']; },keepSelection);
      assert.equal(await page.evaluate(() => window.roomTest.previewLatestBackup()),true);
      await page.waitForFunction(() => !!window.roomTest.roomRestorePreview && !window.roomTest.isSending);
      assert.equal(await page.evaluate(() => window.roomTest.restorePreviewedBackup()),true);
      await stored(page,keepSelection ? "room-a" : "room-b");
    });
  }
  await scenario("new Room selection is saved", {}, async page => {
    await hydrate(page,"moe-dev-room");
    await page.evaluate(() => window.roomTest.createRoom());
    await page.waitForFunction(() => window.roomTest.activeRoom.id.startsWith('room-'));
    await stored(page,await page.evaluate(() => window.roomTest.activeRoom.id));
  });
  console.log(`${passed} scenarios passed`);
} finally {
  await browser?.close();
  await server.close();
}
