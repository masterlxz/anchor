import 'package:path/path.dart';
import 'package:sqflite/sqflite.dart';

/// Banco local do portfolio — 7 classes de ativo (Ação BR/FII/ETF BR/BDR via
/// `YahooQuoteService` com sufixo `.SA`, Ação internacional/Metal via Yahoo
/// sem sufixo, Cripto via CoinGecko — ver `QuoteDispatcher`) e 5 tipos de
/// transação (compra/venda/aporte/retirada/provento — ver
/// `TransactionTypeMeta`). Sem custódia/`transferencia`/renda fixa/campos
/// manuais ainda (ver `project/PHASE.md`, Fase 11 item 11.2).
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
      version: 3,
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
            asset_id INTEGER REFERENCES assets(id) ON DELETE CASCADE,
            transaction_type TEXT NOT NULL,
            quantity REAL,
            unit_price REAL,
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
        if (oldVersion < 3) {
          // `aporte`/`retirada` não referenciam ativo e `provento` não tem
          // quantidade/preço unitário — SQLite não altera constraint NOT
          // NULL via ALTER TABLE, então a tabela é recriada com o schema
          // relaxado e os dados existentes copiados.
          await db.execute('''
            CREATE TABLE portfolio_transactions_new (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              asset_id INTEGER REFERENCES assets(id) ON DELETE CASCADE,
              transaction_type TEXT NOT NULL,
              quantity REAL,
              unit_price REAL,
              total_value REAL NOT NULL,
              transaction_date TEXT NOT NULL,
              notes TEXT,
              created_at TEXT NOT NULL
            )
          ''');
          await db.execute('''
            INSERT INTO portfolio_transactions_new
            SELECT id, asset_id, transaction_type, quantity, unit_price,
                   total_value, transaction_date, notes, created_at
            FROM portfolio_transactions
          ''');
          await db.execute('DROP TABLE portfolio_transactions');
          await db.execute(
              'ALTER TABLE portfolio_transactions_new RENAME TO portfolio_transactions');
        }
      },
    );
  }
}
