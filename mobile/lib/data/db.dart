import 'package:path/path.dart';
import 'package:sqflite/sqflite.dart';

/// Banco local do portfolio — 7 classes de ativo (Ação BR/FII/ETF BR/BDR via
/// `YahooQuoteService` com sufixo `.SA`, Ação internacional/Metal via Yahoo
/// sem sufixo, Cripto via CoinGecko — ver `QuoteDispatcher`). Sem
/// custódia/renda fixa/campos manuais ainda (ver `project/PHASE.md`, Fase
/// 11 item 11.2).
///
/// Ao contrário do desktop (que exige `sea-orm-cli migrate up` manual), as
/// tabelas nascem sozinhas no `onCreate`, na primeira abertura — não existe
/// usuário/DBA rodando migração à mão num app mobile. `onUpgrade` cobre só
/// quem já tinha o app instalado antes de uma coluna nova aparecer.
class AppDatabase {
  static Database? _instance;

  static Future<Database> instance() async {
    _instance ??= await _open();
    return _instance!;
  }

  static Future<Database> _open() async {
    final path = join(await getDatabasesPath(), 'anchor_mobile.db');
    return openDatabase(
      path,
      version: 2,
      onCreate: (db, version) async {
        await db.execute('''
          CREATE TABLE assets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ticker TEXT NOT NULL,
            name TEXT NOT NULL,
            asset_class TEXT NOT NULL,
            currency TEXT NOT NULL,
            created_at TEXT NOT NULL,
            external_id TEXT
          )
        ''');
        await db.execute('''
          CREATE TABLE portfolio_transactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
            transaction_type TEXT NOT NULL,
            quantity REAL NOT NULL,
            unit_price REAL NOT NULL,
            total_value REAL NOT NULL,
            transaction_date TEXT NOT NULL,
            notes TEXT,
            created_at TEXT NOT NULL
          )
        ''');
      },
      onUpgrade: (db, oldVersion, newVersion) async {
        if (oldVersion < 2) {
          await db.execute('ALTER TABLE assets ADD COLUMN external_id TEXT');
        }
      },
    );
  }
}
