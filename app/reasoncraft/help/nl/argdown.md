# Argdown gebruiken

**Argdown** is een manier om argumenten als platte tekst op te schrijven, die door verschillende programma's voor argumentkaarten wordt gebruikt. ReasonCraft kan een kaart als Argdown bewaren en Argdown-bestanden lezen. Zo kun je kaarten tussen programma's uitwisselen of een kaart als tekst schrijven.

## Een kaart als Argdown bewaren

Exporteren > **Argdown** bewaart de hele kaart, ook wat bij Niet verbonden staat. Het bestand bevat alles van de opbouw en de beoordelingen, zodat je bij het teruglezen dezelfde kaart krijgt. Afbeeldingen gaan niet mee; de export meldt dat als de kaart er een heeft. Ook de breedte van vakken, de richting van de kaart, de fragmenten en het betoog staan er niet in.

## Zo ziet een kaart eruit in Argdown

- Uitspraken worden geschreven als `[Titel]: tekst`. ReasonCraft maakt elke titel van de eerste woorden van het vak.
- Elk blok wordt een argument, met de titel `R1`, `R2`, … voor redenen en `O1`, `O2`, … voor bezwaren. De premissen staan genummerd `(1)`, `(2)` boven een lijn `----`, de conclusie eronder.
- `<+` betekent "steunt" en `<-` "spreekt tegen"; `<_` is een bezwaar tegen een heel blok.
- Beoordelingen, soorten bronnen, notities, links en opmerkingen staan als gegevens tussen `{` en `}`.

## Een Argdown-bestand lezen

Bestand > **Argdown importeren…** leest een Argdown-bestand in een nieuw tabblad en deelt het automatisch in. Het bestand zelf wordt nooit gewijzigd: gebruik Opslaan als om de kaart als ReasonCraft-bestand te bewaren.

Een bestand dat ReasonCraft schreef, levert bij het lezen dezelfde kaart op. Bestanden van elders kunnen delen van Argdown gebruiken die een kaart niet kent, zoals tegenspraak, tags, afleidingsregels of één uitspraak op meerdere plaatsen. ReasonCraft past die delen aan of laat ze weg, en toont een overzicht met elk geval en het regelnummer.

- De uitspraak waar de meeste andere naar verwijzen, wordt de bewering; andere losse bomen gaan naar Niet verbonden.
- Bestanden mogen tot 2 MB groot zijn, met tot 5000 uitspraken.

## Lees verder

- [Exporteren en afdrukken](exports-and-print)
- [Bestanden en herstel](files-and-recovery)
