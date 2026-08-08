import 'package:path/path.dart';
import 'package:sqflite/sqflite.dart';

/// Banco local do portfolio — só as 4 classes de ativo que reaproveitam
/// `YahooQuoteService` (Ação BR/FII/ETF BR/BDR, mesmo sufixo `.SA`). Sem
/// custódia/renda fixa/campos manuais ainda (ver `project/PHASE.md`, Fase
/// 11 item 11.2).
///
/// Ao contrário do desktop (que exige `sea-orm-cli migrate up` manual), as
/// tabelas nascem sozinhas no `onCreate`, na primeira abertura — não existe
/// usuário/DBA rodando migração à mão num app mobile.
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
      version: 1,
      onCreate: (db, version) async {
        await db.execute('''
          CREATE TABLE assets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ticker TEXT NOT NULL,
            name TEXT NOT NULL,
            asset_class TEXT NOT NULL,
            currency TEXT NOT NULL,
            created_at TEXT NOT NULL
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
    );
  }
}
