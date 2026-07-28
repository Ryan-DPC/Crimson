const fs = require('fs');
const path = require('path');

async function fix() {
  const resp = await fetch('https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/default/v1/perks.json');
  const perks = await resp.json();
  
  const runesPath = path.resolve(__dirname, '../crimson/src/assets/data/runesData.json');
  console.log('Reading runesData from:', runesPath);
  
  if (!fs.existsSync(runesPath)) {
    console.error('Error: runesData.json not found at', runesPath);
    process.exit(1);
  }
  
  const runesData = JSON.parse(fs.readFileSync(runesPath, 'utf8'));
  
  for (const tree of runesData) {
    for (const slot of tree.slots) {
      for (const rune of slot.runes) {
        // Handle removals/replacements
        if (rune.id === 9101) { // Overheal -> Absorb Life
            const cdragon = perks.find(p => p.name === 'Absorb Life');
            if (cdragon) {
                rune.id = cdragon.id;
                rune.icon = cdragon.iconPath.replace('/lol-game-data/assets/v1/', '');
                console.log('Replaced Overheal with Absorb Life (ID:', rune.id, ')');
            }
        } else if (rune.id === 9105) { // Legend: Tenacity -> Legend: Haste
            const cdragon = perks.find(p => p.name === 'Legend: Haste');
            if (cdragon) {
                rune.id = cdragon.id;
                rune.icon = cdragon.iconPath.replace('/lol-game-data/assets/v1/', '');
                console.log('Replaced Legend: Tenacity with Legend: Haste (ID:', rune.id, ')');
            }
        } else {
            const actual = perks.find(p => p.id === rune.id);
            if (actual) {
                rune.icon = actual.iconPath.replace('/lol-game-data/assets/v1/', '');
            } else {
                console.log("Not found in CDragon:", rune.id);
            }
        }
      }
    }
  }
  
  fs.writeFileSync(runesPath, JSON.stringify(runesData, null, 4));
  console.log("Done checking icons.");
}

fix();
