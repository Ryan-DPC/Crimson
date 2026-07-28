# État des lieux - Bogues et Améliorations (Crimson v3.0.6)

Suite au passage à la nouvelle architecture Monorepo sur le lecteur `F:`, voici la liste des problèmes identifiés qu'il nous reste à corriger. Ce fichier servira de point de référence pour nos prochaines sessions.

## 1. Plugin Spotify - Clignotement du logo
* **Symptôme :** Le plugin sur le Stream Deck clignote parfois en affichant un logo Spotify générique (blanc et vert) avant de revenir à l'image normale.
* **Piste de résolution :** Lors de la synchronisation de l'état ou du rafraîchissement, le système pousse probablement une image par défaut ou subit une latence. Il faut vérifier la logique d'envoi d'images dans `streamdock.rs` et `ws.rs` pour éviter ce clignotement intermédiaire.

## 2. Plugin Spotify - Smart Shuffle (3ème état)
* **Symptôme :** Le bouton Shuffle ne fonctionne pas lorsqu'on arrive sur le 3ème état (le "Smart Shuffle").
* **Piste de résolution :** L'API de Spotify a une façon particulière de gérer le Smart Shuffle (souvent différent d'un simple booléen `true/false`). Il faut retrouver le correctif que nous avions identifié précédemment et l'appliquer dans la méthode de changement de Shuffle de `server/src/spotify.rs` ou la gestion du payload côté WebSocket.

## 3. League of Legends (LCU) - Auto-Accept et Pick & Ban
* **Symptôme :** Les fonctionnalités d'Auto-Accept ne marchent plus. Les Picks et Bans peuvent être configurés sur l'interface web, mais ne s'exécutent pas dans le client League of Legends.
* **Piste de résolution :** Avec la séparation du serveur de commandes et de l'application Tauri, la boucle d'écoute LCU (`lcu_commands/src/lcu.rs`) ne reçoit probablement plus correctement les mises à jour d'état du WebSocket. Il faut rétablir la communication entre l'état local (Tauri/Frontend) et la boucle d'exécution LCU.

## 4. Frontend - Interface et CSS
* **Symptôme :** Problèmes de design visuel, notamment des éléments qui se superposent (la section "Auto Selection" et la grille des champions sont trop à l'étroit).
* **Piste de résolution :** Revoir les styles CSS (Tailwind ou CSS pur) dans les composants React concernés (ex: `AutoSelection.tsx`, `MatchHistory.tsx`, etc.) pour utiliser correctement Flexbox/Grid et s'assurer que l'interface est responsive et aérée.
