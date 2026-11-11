//! The texts that the HD text draws, and the places in the pictures where they go.
//! The tables are keyed by the language letter of the file names: F, G, I, S and U.

/// The names of the places for the chapter titles, as a language letter, a chapter code and the text to draw.
pub const PLACES: &[(char, &str, &str)] = &[
    ('F', "B009", "Cour du château de Plegia"),
    ('F', "B019", "Abords du chât. de Valm"),
    ('F', "BX002", "Secteur des frères bandits"),
    ('F', "BX004", "Repaire des frères bandits"),
    ('F', "BX007", "Ruines du temple de Mila"),
    ('F', "BX017", "Terres du Dragon Divin"),
    ('F', "BX019", "Enclume du Conquérant"),
    ('G', "B001", "Westlich von Ylisstol"),
    ('G', "B003", "Die Mauerfestung"),
    ('G', "B007", "Der Halsbrecherpass"),
    ('G', "B010", "Mitten im Morast"),
    ('G', "B015", "Valmischer Hafen"),
    ('G', "B019", "Vor Schloss Valm"),
    ('G', "B023", "Die Tafel des Drachen"),
    ('G', "B025", "Berg des Ursprungs"),
    ('G', "BX001", "Das Fort in der Ferne"),
    ('G', "BX002", "Domäne der Zwillinge"),
    ('G', "BX004", "Versteck der Zwillinge"),
    ('G', "BX005", "Des Weisen Weiler"),
    ('G', "BX007", "Ruinen des Milaschreins"),
    ('G', "BX012", "Die Ruinen der Zeit"),
    ('G', "BX013", "Ende der Ordnung"),
    ('G', "BX015", "Burg der Entführer"),
    ('G', "BX017", "Gelände der Wyrmgöttin"),
    ('G', "BX018", "Thron des Seekönigs"),
    ('G', "BX019", "Schleifstein des Eroberers"),
    ('G', "BX022", "Der Quell der Wahrheit"),
    ('G', "BX023", "Garten der Riesen"),
    ('I', "B001", "A ovest d'Ylisstol"),
    ('I', "B002", "La strada del nord"),
    ('I', "B005", "Il passo di confine"),
    ('I', "B007", "Un valico pericoloso"),
    ('I', "B009", "La corte plegiana"),
    ('I', "B010", "Le Acquemorte"),
    ('I', "B013", "L'isola dei Morti"),
    ('I', "B014", "La strada sul mare"),
    ('I', "B016", "L'albero di Mila"),
    ('I', "B018", "La Fornace del Demone"),
    ('I', "B020", "Il castello di Valm"),
    ('I', "B021", "Il castello di Plegia"),
    ('I', "B023", "L'altare del Drago"),
    ('I', "BX002", "La terra dei gemelli"),
    ('I', "BX003", "Un villaggio tranquillo"),
    ('I', "BX004", "Il covo dei gemelli"),
    ('I', "BX005", "La dimora del saggio"),
    ('I', "BX006", "La fortezza"),
    ('I', "BX007", "Il tempio di Mila"),
    ('I', "BX008", "Il luogo del duello"),
    ('I', "BX009", "La foresta lussureggiante"),
    ('I', "BX010", "La fortezza dei mercenari"),
    ('I', "BX012", "Le rovine del tempo"),
    ('I', "BX013", "Nella terra di nessuno"),
    ('I', "BX014", "Un'oasi nel deserto"),
    ('I', "BX015", "I bastioni del bandito"),
    ('I', "BX016", "La casa dei morti"),
    ('I', "BX017", "Le terre del Drago Divino"),
    ('I', "BX018", "Il castello del re dei mari"),
    ('I', "BX019", "La pietra del Conquistatore"),
    ('I', "BX020", "Un villaggio di montagna"),
    ('I', "BX021", "Le tombe dei guerrieri"),
    ('I', "BX022", "La fonte della verità"),
    ('I', "BX023", "Il giardino dei giganti"),
    ('S', "B019", "Castillo de Valm (proximidades)"),
    ('S', "B022", "Altar del Dragón (exterior)"),
    ('S', "BX015", "Fortaleza del secuestrador"),
    ('S', "BX016", "Hogar de las almas errantes"),
];

/// The names of the chapters for the chapter titles, as a language letter, a chapter code and the text to draw.
pub const NAMES: &[(char, &str, &str)] = &[
    ('F', "010", "Jour nouveau"),
    ('U', "X018", "The Dead King's Lament"),
];

/// The keys of the banner texts, in the order of the texts of each language in `BANNER_TEXTS`.
/// The mission banners carry the index of their texture, as in `Mission01.1`.
pub const BANNER_KEYS: &[&str] = &["PlayerPhase", "EnemyPhase", "AllyPhase", "Classchange", "Levelup", "Mission01.1", "Mission01.2", "Mission02.1", "StageClear", "YouLose", "YouRetire", "YouWin", "GameOver"];

/// The texts of the banners for each language, in the order of `BANNER_KEYS`.
pub const BANNER_TEXTS: &[(char, &[&str])] = &[
    ('F', &["PHASE JOUEUR", "PHASE ENNEMIS", "PHASE ALLIÉS", "CHANGEMENT DE CLASSE !", "CHANGEMENT DE NIVEAU !", "Vaincre le boss", "VICTOIRE", "Décimer l'ennemi", "Terminé !", "Défaite !", "Abandon !", "Victoire !", "Game Over"]),
    ('G', &["SPIELERPHASE", "GEGNERPHASE", "VERBÜNDETENPHASE", "NEUE KLASSE!", "LEVEL UP!", "Boss besiegen", "SIEG", "Alle besiegen", "Abgeschlossen!", "Verloren...", "Aufgegeben!", "Gewonnen!", "Game Over"]),
    ('I', &["FASE GIOCATORE", "FASE NEMICI", "FASE ALLEATI", "CAMBIO CLASSE!", "NUOVO LIVELLO!", "sconfiggi il comandante", "Vinci se", "sgomini i nemici", "LIVELLO COMPLETATO!", "Hai perso", "Ritirata", "Hai vinto!", "Game Over"]),
    ('S', &["TURNO DEL JUGADOR", "TURNO DEL ENEMIGO", "TURNO DE LOS ALIADOS", "¡CAMBIO DE CLASE!", "+1 NIVEL", "Derrota al comandante", "PARA GANAR:", "Vence a todos los enemigos", "¡Fase superada!", "Has perdido", "Retirada", "¡Has ganado!", "Fin de la partida"]),
    ('U', &["PLAYER PHASE", "ENEMY PHASE", "ALLY PHASE", "CLASS CHANGE!", "LEVEL UP!", "Defeat commander", "VICTORY", "Rout the enemy", "STAGE COMPLETE!", "You Lose", "You Yield", "You Win!", "Game Over"]),
];

/// The banners to redraw, as the name of the telop file, the index of the texture in the file and the kind of the banner.
/// The kind is "plain", "glow" or "stone".
pub const BANNER_SPECS: &[(&str, usize, &str)] = &[
    ("PlayerPhase", 1, "plain"),
    ("PlayerPhase", 3, "glow"),
    ("EnemyPhase", 1, "plain"),
    ("EnemyPhase", 3, "glow"),
    ("AllyPhase", 1, "plain"),
    ("AllyPhase", 3, "glow"),
    ("Classchange", 1, "plain"),
    ("Classchange", 2, "glow"),
    ("Levelup", 1, "plain"),
    ("Levelup", 2, "glow"),
    ("Mission01", 1, "plain"),
    ("Mission01", 2, "plain"),
    ("Mission02", 1, "plain"),
    ("StageClear", 4, "plain"),
    ("YouLose", 1, "stone"),
    ("YouLose", 2, "glow"),
    ("YouRetire", 1, "plain"),
    ("YouWin", 1, "plain"),
    ("GameOver", 1, "plain"),
];

/// A label in a picture of the status screens that is drawn again.
pub struct Label {
    /// The box of the label in the picture, as left, top, right and bottom in pixels.
    pub box_: [usize; 4],
    /// The text to draw.
    pub text: &'static str,
    /// The name of the group of the label, or empty. Labels of a group are drawn at the same scale.
    pub group: &'static str,
    /// Whether the label is a button, which is drawn differently from a plain label.
    pub button: bool,
    /// The colour of the letters of a button, "light" or "dark", or empty to find it from the picture.
    pub letters: &'static str,
    /// Whether the label has a shine.
    pub shine: bool,
    /// The name of the size group that sets the scale of the label, or empty to use the group.
    pub size: &'static str,
}

/// The labels of the status atlases, as a language letter, the offset of the texture in the file and the labels of that texture.
pub const LABELS: &[(char, usize, &[Label])] = &[
    ('F', 0x20100, &[
        Label { box_: [192, 4, 221, 21], text: "Atq", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 22, 221, 39], text: "Prc", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 40, 221, 57], text: "Crit", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 58, 221, 75], text: "Esq", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [69, 43, 99, 58], text: "NIV", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [129, 43, 159, 58], text: "EXP", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [69, 59, 99, 74], text: "PV", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [3, 81, 35, 98], text: "Frc", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 99, 35, 116], text: "Mag", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 117, 35, 134], text: "Tec", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 135, 35, 152], text: "Vit", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 153, 35, 170], text: "Cha", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 171, 35, 188], text: "Déf", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 189, 35, 205], text: "Rés", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 209, 359, 226], text: "Père", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 209, 394, 226], text: "Enfant", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 233, 359, 250], text: "Mère", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 233, 399, 250], text: "Mari", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [401, 233, 439, 250], text: "Femme", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [441, 233, 479, 250], text: "Proche", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [419, 3, 451, 34], text: "Détail.", group: "button dim", button: true, letters: "", shine: false, size: "buttons" },
        Label { box_: [419, 44, 451, 75], text: "Détail.", group: "button pressed", button: true, letters: "light", shine: false, size: "buttons" },
        Label { box_: [419, 82, 451, 114], text: "Détail.", group: "button", button: true, letters: "dark", shine: false, size: "buttons" },
    ]),
    ('F', 0x40100, &[
        Label { box_: [193, 5, 243, 20], text: "Éval.", group: "level", button: false, letters: "", shine: false, size: "" },
        Label { box_: [193, 66, 280, 82], text: "Faiblesses", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [193, 106, 280, 121], text: "Techniques", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [135, 182, 280, 199], text: "Niveau d'arme", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [69, 43, 99, 58], text: "NIV", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [69, 59, 99, 74], text: "PV", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [321, 209, 359, 226], text: "Père", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 209, 394, 226], text: "Enfant", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 233, 359, 250], text: "Mère", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 233, 399, 250], text: "Mari", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [401, 233, 439, 250], text: "Femme", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [441, 233, 479, 250], text: "Proche", group: "family", button: false, letters: "", shine: false, size: "" },
    ]),
    ('G', 0x20100, &[
        Label { box_: [192, 4, 221, 21], text: "Agr.", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 22, 221, 39], text: "Trf.", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 40, 221, 57], text: "Krt.", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 58, 221, 75], text: "Aw.", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [69, 43, 99, 58], text: "Lv.", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [129, 43, 159, 58], text: "Erf.", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [69, 59, 99, 74], text: "KP", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [3, 81, 35, 98], text: "Stä.", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 99, 35, 116], text: "Mag.", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 117, 35, 134], text: "Fäh.", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 135, 35, 152], text: "Ges.", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 153, 35, 170], text: "Glk.", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 171, 35, 188], text: "Ver.", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 189, 35, 205], text: "Res.", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 209, 359, 226], text: "Vater", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 209, 394, 226], text: "Kind", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 233, 359, 250], text: "Mutter", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 233, 399, 250], text: "Ehemann", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [401, 233, 439, 250], text: "Ehefrau", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [441, 233, 479, 250], text: "Partner", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [419, 3, 451, 34], text: "Info", group: "button dim", button: true, letters: "", shine: false, size: "buttons" },
        Label { box_: [419, 44, 451, 75], text: "Info", group: "button pressed", button: true, letters: "light", shine: false, size: "buttons" },
        Label { box_: [419, 82, 451, 114], text: "Info", group: "button", button: true, letters: "dark", shine: false, size: "buttons" },
    ]),
    ('G', 0x40100, &[
        Label { box_: [193, 5, 243, 20], text: "Macht", group: "level", button: false, letters: "", shine: false, size: "" },
        Label { box_: [193, 66, 280, 82], text: "Verwundbarkeit", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [193, 106, 280, 121], text: "Fähigkeiten", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [135, 182, 280, 199], text: "Waffen-Level", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [69, 43, 99, 58], text: "Lv.", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [69, 59, 99, 74], text: "KP", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [321, 209, 359, 226], text: "Vater", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 209, 394, 226], text: "Kind", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 233, 359, 250], text: "Mutter", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 233, 399, 250], text: "Ehemann", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [401, 233, 439, 250], text: "Ehefrau", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [441, 233, 479, 250], text: "Partner", group: "family", button: false, letters: "", shine: false, size: "" },
    ]),
    ('I', 0x20100, &[
        Label { box_: [192, 4, 221, 21], text: "Att", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 22, 221, 39], text: "Clp", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 40, 221, 57], text: "Trpl", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 58, 221, 75], text: "Scv", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [69, 43, 99, 58], text: "Liv", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [129, 43, 159, 58], text: "Esp", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [69, 59, 99, 74], text: "PS", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [3, 81, 35, 98], text: "Forz", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 99, 35, 116], text: "Mag", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 117, 35, 134], text: "Abil", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 135, 35, 152], text: "Vel", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 153, 35, 170], text: "Fort", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 171, 35, 188], text: "Dif", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 189, 35, 205], text: "Res", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 209, 359, 226], text: "Padre", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 209, 394, 226], text: "Erede", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 233, 359, 250], text: "Madre", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 233, 399, 250], text: "Sposo", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [401, 233, 439, 250], text: "Sposa", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [441, 233, 479, 250], text: "Partner", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [419, 3, 451, 34], text: "DATI", group: "button dim", button: true, letters: "", shine: false, size: "buttons" },
        Label { box_: [419, 44, 451, 75], text: "DATI", group: "button pressed", button: true, letters: "light", shine: false, size: "buttons" },
        Label { box_: [419, 82, 451, 114], text: "DATI", group: "button", button: true, letters: "dark", shine: false, size: "buttons" },
    ]),
    ('I', 0x40100, &[
        Label { box_: [193, 5, 243, 20], text: "Valore", group: "level", button: false, letters: "", shine: false, size: "" },
        Label { box_: [193, 66, 280, 82], text: "Vulnerabilità", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [193, 106, 280, 121], text: "Abilità", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [135, 182, 280, 199], text: "Livello armi", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [69, 43, 99, 58], text: "Liv", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [69, 59, 99, 74], text: "PS", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [321, 209, 359, 226], text: "Padre", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 209, 394, 226], text: "Erede", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 233, 359, 250], text: "Madre", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 233, 399, 250], text: "Sposo", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [401, 233, 439, 250], text: "Sposa", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [441, 233, 479, 250], text: "Partner", group: "family", button: false, letters: "", shine: false, size: "" },
    ]),
    ('S', 0x20100, &[
        Label { box_: [192, 4, 221, 21], text: "Atc", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 22, 221, 39], text: "Gol", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 40, 221, 57], text: "Crí", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 58, 221, 75], text: "Esq", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [69, 43, 99, 58], text: "Niv", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [129, 43, 159, 58], text: "Exp", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [69, 59, 99, 74], text: "PV", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [3, 81, 35, 98], text: "Fue", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 99, 35, 116], text: "Mag", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 117, 35, 134], text: "Hab", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 135, 35, 152], text: "Vel", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 153, 35, 170], text: "Sue", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 171, 35, 188], text: "Def", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 189, 35, 205], text: "Res", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 209, 359, 226], text: "Padre", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 209, 394, 226], text: "Hijos", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 233, 359, 250], text: "Madre", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 233, 399, 250], text: "Cónyuge", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [401, 233, 439, 250], text: "Cónyuge", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [441, 233, 479, 250], text: "Pareja", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [419, 3, 451, 34], text: "Ficha", group: "button dim", button: true, letters: "", shine: false, size: "buttons" },
        Label { box_: [419, 44, 451, 75], text: "Ficha", group: "button pressed", button: true, letters: "light", shine: false, size: "buttons" },
        Label { box_: [419, 82, 451, 114], text: "Ficha", group: "button", button: true, letters: "dark", shine: false, size: "buttons" },
    ]),
    ('S', 0x40100, &[
        Label { box_: [193, 5, 243, 20], text: "Valoración", group: "level", button: false, letters: "", shine: false, size: "" },
        Label { box_: [193, 66, 280, 82], text: "Debilidades", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [193, 106, 280, 121], text: "Habilidades", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [135, 182, 280, 199], text: "Destreza", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [69, 43, 99, 58], text: "Niv", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [69, 59, 99, 74], text: "PV", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [321, 209, 359, 226], text: "Padre", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 209, 394, 226], text: "Hijos", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 233, 359, 250], text: "Madre", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 233, 399, 250], text: "Cónyuge", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [401, 233, 439, 250], text: "Cónyuge", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [441, 233, 479, 250], text: "Pareja", group: "family", button: false, letters: "", shine: false, size: "" },
    ]),
    ('U', 0x20100, &[
        Label { box_: [192, 4, 221, 21], text: "Atk", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 22, 221, 39], text: "Hit", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 40, 221, 57], text: "Crit", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [192, 58, 221, 75], text: "Avo", group: "combat", button: false, letters: "", shine: false, size: "" },
        Label { box_: [69, 43, 99, 58], text: "LV", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [129, 43, 159, 58], text: "EXP", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [69, 59, 99, 74], text: "HP", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [3, 81, 35, 98], text: "Str", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 99, 35, 116], text: "Mag", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 117, 35, 134], text: "Skill", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 135, 35, 152], text: "Spd", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 153, 35, 170], text: "Lck", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 171, 35, 188], text: "Def", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [3, 189, 35, 205], text: "Res", group: "stats", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 209, 359, 226], text: "Father", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 209, 394, 226], text: "Child", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 233, 359, 250], text: "Mother", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 233, 399, 250], text: "Husband", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [401, 233, 439, 250], text: "Wife", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [441, 233, 479, 250], text: "Companion", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [419, 3, 451, 34], text: "FULL", group: "button dim", button: true, letters: "", shine: false, size: "buttons" },
        Label { box_: [419, 44, 451, 75], text: "FULL", group: "button pressed", button: true, letters: "light", shine: false, size: "buttons" },
        Label { box_: [419, 82, 451, 114], text: "FULL", group: "button", button: true, letters: "dark", shine: false, size: "buttons" },
    ]),
    ('U', 0x40100, &[
        Label { box_: [193, 5, 243, 20], text: "Rating", group: "level", button: false, letters: "", shine: false, size: "" },
        Label { box_: [193, 66, 280, 82], text: "Weaknesses", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [193, 106, 280, 121], text: "Skills", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [135, 182, 280, 199], text: "Weapon Levels", group: "headers", button: false, letters: "", shine: false, size: "" },
        Label { box_: [69, 43, 99, 58], text: "LV", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [69, 59, 99, 74], text: "HP", group: "level", button: false, letters: "", shine: true, size: "" },
        Label { box_: [321, 209, 359, 226], text: "Father", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 209, 394, 226], text: "Child", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [321, 233, 359, 250], text: "Mother", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [361, 233, 399, 250], text: "Husband", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [401, 233, 439, 250], text: "Wife", group: "family", button: false, letters: "", shine: false, size: "" },
        Label { box_: [441, 233, 479, 250], text: "Companion", group: "family", button: false, letters: "", shine: false, size: "" },
    ]),
];

/// The "press any button" text of the title screen for each language.
pub const PRESS_ANY_BUTTON: &[(char, &str)] = &[('F', "Appuyez sur un bouton"), ('G', "KNOPF DRÜCKEN"), ('I', "PREMI UN PULSANTE"), ('S', "PULSA UN BOTÓN"), ('U', "PRESS ANY BUTTON")];

/// The eight texts of the main menu for each language, in the order of the boxes of the menu.
pub const MAIN_MENU: &[(char, [&str; 8])] = &[
    ('F', ["Continuer", "Signet", "Nouvelle partie", "Classique", "Débutant", "Copier", "Effacer", "Extras"]),
    ('G', ["Fortsetzen", "Lesezeichen", "Neues Spiel", "Klassisch", "Anfänger", "Kopieren", "Löschen", "Extras"]),
    ('I', ["Partite salvate", "Segnalibro", "Nuova partita", "Classica", "Principiante", "Copia", "Cancella", "Extra"]),
    ('S', ["Continuar", "Reanudar batalla", "Nueva partida", "Clásico", "Novato", "Copiar", "Borrar", "Extras"]),
    ('U', ["Continue", "Bookmark", "New Game", "Classic", "Newcomer", "Copy", "Delete", "Extras"]),
];

/// The copyright line of the title screen.
pub const COPYRIGHT: &str = "©2012-2013 Nintendo / INTELLIGENT SYSTEMS";

/// The abbreviations in the class names of the game, as a language letter, the short form and the full form that replaces it.
pub const CLASS_SPELLINGS: &[(char, &str, &str)] = &[
    ('F', "Ch. ", "Chevalier "),
    ('I', "Cav. ", "Cavaliere "),
    ('I', "Sac. guerriero", "Sacerdote guerriero"),
    ('I', "Sac. guerriera", "Sacerdotessa guerriera"),
    ('I', "Gran Lord", "Gran lord"),
    ('S', "Inf. ", "Infantería "),
    ('S', "Cab. ", "Caballero "),
    ('S', "Sacerd. ", "Sacerdotisa "),
];

/// The words of the three large buttons of the world map for each language: close, story and shop.
pub const GMAP_WORDS: &[(char, [&str; 3])] = &[
    ('F', ["Fermer", "Histoire", "Magasin"]),
    ('G', ["Schl.", "STORY", "HÄNDLER"]),
    ('I', ["Chiudi", "STORIA", "EMPORIO"]),
    ('S', ["Cerrar", "HISTORIA", "TIENDA"]),
    ('U', ["Close", "STORY", "SHOP"]),
];

/// The five pill-shaped buttons of the world map for each language.
/// The numbers are the horizontal positions in the picture where the pills start and end, which differ with the length of the words.
pub const GMAP_PILLS: &[(char, [&str; 5], [usize; 8])] = &[
    ('F', ["Retour", "Halte", "Menu", "Examiner", "Atteindre objectif"], [144, 24, 56, 73, 122, 149, 250, 0]),
    ('G', ["Zurück", "Stopp", "Menü", "Überblick", "Zum Ziel gehen"], [142, 28, 61, 78, 135, 161, 250, 0]),
    ('I', ["Indietro", "Stop", "Menu", "Zoom", "Vai all'obiettivo"], [143, 29, 64, 89, 133, 162, 250, 0]),
    ('S', ["Acercar", "Parar", "Menú", "Alejar", "Siguiente capítulo"], [143, 28, 61, 78, 116, 143, 250, 0]),
    ('U', ["Back", "Halt", "Menu", "Survey", "Head to Objective"], [146, 28, 62, 82, 127, 154, 250, 0]),
];

/// The horizontal positions of the button icons of the world map for each language, as pairs of left and right edges.
pub const GMAP_ICONS: &[(char, [usize; 8])] = &[
    ('F', [9, 22, 59, 71, 125, 147, 131, 142]),
    ('G', [13, 26, 64, 76, 138, 160, 128, 139]),
    ('I', [13, 26, 75, 87, 136, 158, 129, 140]),
    ('S', [13, 26, 64, 76, 119, 141, 128, 139]),
    ('U', [13, 26, 68, 80, 130, 152, 131, 142]),
];

/// The words of the chapter overview of the status screen for each language, indexed by the overview table of the words module.
pub const OVERVIEW_WORDS: &[(char, [&str; 9])] = &[
    ('F', ["Victoire", "Tour", "NIV", "EXP", "PV", "Vaincre", "le boss", "Décimer", "l'ennemi"]),
    ('G', ["Sieg", "Zug", "Lv.", "Erf.", "KP", "Boss", "besiegen", "Alle", "besiegen"]),
    ('I', ["Vinci se", "Turno", "Liv", "Esp", "PS", "sconfiggi il", "comandante", "sgomini", "i nemici"]),
    ('S', ["Victoria", "Turno", "Niv", "Exp", "PV", "Derrota al", "comandante", "Vence a todos", "los enemigos"]),
    ('U', ["Victory", "Turn", "LV", "EXP", "HP", "Defeat", "commander", "Rout the", "enemy"]),
];

/// The word of the close button of the tutorials for each language.
pub const TUTORIAL_WORDS: &[(char, &str)] = &[('F', "Fermer"), ('G', "Schließen"), ('I', "Chiudi"), ('S', "Cerrar"), ('U', "Close")];

/// The words of the battle forecast and the help marks for each language, indexed by the forecast table of the words module.
pub const FORECAST_WORDS: &[(char, [&str; 13])] = &[
    ('F', ["Précision", "PV", "Atq", "Prc", "Crit", "Rés", "INFO", "NOUV.", "FINI!", "MAX", "↑", "x2", "x4"]),
    ('G', ["Trf.-Ch.", "KP", "Agr.", "Trf.", "Krt.", "Res.", "Hilfe", "Neu!", "FERTIG!", "MAX", "+", "x2", "x4"]),
    ('I', ["Prob. Clp", "PS", "Att", "Clp", "Trpl", "Res", "INFO", "NUOVO!", "FINE!", "MAX", "SU", "x2", "x4"]),
    ('S', ["Acierto", "PV", "Atc", "Gol", "Crí", "Res", "INFO", "NUEVO", "¡LISTO!", "Máx", "MÁS", "x2", "x4"]),
    ('U', ["Hit Rate", "HP", "Atk", "Hit", "Crit", "Res", "HELP", "NEW!", "CLEAR!", "MAX", "UP", "x2", "x4"]),
];

/// The words of the unit information screen for each language, indexed by the unit table of the words module.
pub const UNIT_WORDS: &[(char, [&str; 10])] = &[
    ('F', ["Bases", "Carac.", "Combat", "Armes", "Techniques", "NIV", "Vous", "Aucun", "signal", "VS"]),
    ('G', ["Allgemein", "Werte", "Kampf", "Waffen", "Fähigkeiten", "Lv.", "Du", "Kein", "Signal", "Gegen"]),
    ('I', ["Base", "Dati", "Combatt.", "Armi", "Abilità", "Liv", "Tu", "Segn.", "ass.", "VS"]),
    ('S', ["General", "Atributos", "Combate", "Armas", "Habilidades", "Niv", "Tú", "Sin", "señal", "VS"]),
    ('U', ["Basics", "Stats", "Combat", "Weapons", "Skills", "LV", "You", "No", "Signal", "VS"]),
];

/// The six stat names around the radar chart for each language, indexed by the radar table of the words module.
pub const RADAR_WORDS: &[(char, [&str; 6])] = &[
    ('F', ["Frc", "Vit", "Déf", "Rés", "Tec", "Mag"]),
    ('G', ["Stä.", "Ges.", "Ver.", "Res.", "Fäh.", "Mag."]),
    ('I', ["Forz", "Vel", "Dif", "Res", "Abil", "Mag"]),
    ('S', ["Fue", "Vel", "Def", "Res", "Hab", "Mag"]),
    ('U', ["Str", "Spd", "Def", "Res", "Skill", "Mag"]),
];

/// The difficulty names and the short mode names on the cards of the file menu for each language, indexed by the card table of the words module.
pub const CARD_WORDS: &[(char, [&str; 6])] = &[
    ('F', ["Normal", "Difficile", "Expert", "Dément", "DÉB", "CLA"]),
    ('G', ["Normal", "Schwer", "Extrem", "Extrem+", "ANF", "KLA"]),
    ('I', ["Normale", "Difficile", "Folle", "Folle+", "PR", "CL"]),
    ('S', ["Normal", "Difícil", "Extremo", "Brutal", "NOV", "CLÁ"]),
    ('U', ["Normal", "Hard", "Lunatic", "Lunatic+", "NEW", "CLA"]),
];
