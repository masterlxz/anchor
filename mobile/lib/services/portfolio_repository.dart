import '../data/db.dart';
import '../models/asset.dart';
import '../models/asset_valuation.dart';
import '../models/custodia.dart';
import '../models/portfolio_transaction.dart';
import '../models/position.dart';

class PortfolioRepository {
  Future<int> insertAsset(Asset asset) async {
    final db = await AppDatabase.instance();
    final map = asset.toMap()..remove('id');
    return db.insert('assets', map);
  }

  Future<List<Asset>> listAssets() async {
    final db = await AppDatabase.instance();
    final rows = await db.query('assets', orderBy: 'ticker');
    return rows.map(Asset.fromMap).toList();
  }

  Future<void> deleteAsset(int id) async {
    final db = await AppDatabase.instance();
    await db.delete('assets', where: 'id = ?', whereArgs: [id]);
  }

  /// Percentual/valor da participação nunca são gravados direto — sempre
  /// recalculados a partir destes 3 campos (ver `Asset.equityPercentual`),
  /// mesma regra do `update_asset_equity` do desktop.
  Future<void> updateAssetEquity(
    int assetId, {
    double? sharesOwned,
    double? totalShares,
    double? companyValuation,
  }) async {
    final db = await AppDatabase.instance();
    await db.update(
      'assets',
      {
        'equity_shares_owned': sharesOwned,
        'equity_total_shares': totalShares,
        'equity_company_valuation': companyValuation,
      },
      where: 'id = ?',
      whereArgs: [assetId],
    );
  }

  /// `origin` sempre `'manual'` — mesma regra do `add_asset_valuation` do
  /// desktop, nunca exposta como parâmetro pra tela não poder gravar outra
  /// coisa.
  Future<int> insertAssetValuation({
    required int assetId,
    required DateTime valuationDate,
    required double value,
    String? notes,
  }) async {
    final db = await AppDatabase.instance();
    return db.insert(
      'asset_valuations',
      AssetValuation(
        assetId: assetId,
        valuationDate: valuationDate,
        value: value,
        origin: 'manual',
        notes: notes,
        createdAt: DateTime.now(),
      ).toMap()..remove('id'),
    );
  }

  Future<List<AssetValuation>> listAssetValuations(int assetId) async {
    final db = await AppDatabase.instance();
    final rows = await db.query(
      'asset_valuations',
      where: 'asset_id = ?',
      whereArgs: [assetId],
      orderBy: 'valuation_date',
    );
    return rows.map(AssetValuation.fromMap).toList();
  }

  /// `null` quando o ativo nunca recebeu uma avaliação — usado pelo
  /// Portfolio como "preço atual" de `imovel` (quantidade é sempre 1, então
  /// 1×valor = valor, sem precisar de matemática nova).
  Future<AssetValuation?> getLatestValuation(int assetId) async {
    final db = await AppDatabase.instance();
    final rows = await db.query(
      'asset_valuations',
      where: 'asset_id = ?',
      whereArgs: [assetId],
      orderBy: 'valuation_date DESC',
      limit: 1,
    );
    return rows.isEmpty ? null : AssetValuation.fromMap(rows.first);
  }

  Future<void> deleteAssetValuation(int id) async {
    final db = await AppDatabase.instance();
    await db.delete('asset_valuations', where: 'id = ?', whereArgs: [id]);
  }

  Future<int> insertCustodia(Custodia custodia) async {
    final db = await AppDatabase.instance();
    final map = custodia.toMap()..remove('id');
    return db.insert('custodias', map);
  }

  Future<List<Custodia>> listCustodias() async {
    final db = await AppDatabase.instance();
    final rows = await db.query('custodias', orderBy: 'instituicao');
    return rows.map(Custodia.fromMap).toList();
  }

  Future<void> updateCustodia(Custodia custodia) async {
    final db = await AppDatabase.instance();
    await db.update(
      'custodias',
      custodia.toMap()..remove('id'),
      where: 'id = ?',
      whereArgs: [custodia.id],
    );
  }

  Future<void> deleteCustodia(int id) async {
    final db = await AppDatabase.instance();
    await db.delete('custodias', where: 'id = ?', whereArgs: [id]);
  }

  Future<int> insertTransaction(PortfolioTransaction transaction) async {
    final db = await AppDatabase.instance();
    final map = transaction.toMap()..remove('id');
    return db.insert('portfolio_transactions', map);
  }

  Future<List<PortfolioTransaction>> listTransactionsForAsset(
    int assetId,
  ) async {
    final db = await AppDatabase.instance();
    final rows = await db.query(
      'portfolio_transactions',
      where: 'asset_id = ?',
      whereArgs: [assetId],
      orderBy: 'transaction_date',
    );
    return rows.map(PortfolioTransaction.fromMap).toList();
  }

  Future<List<PortfolioTransaction>> listAllTransactions() async {
    final db = await AppDatabase.instance();
    final rows = await db.query(
      'portfolio_transactions',
      orderBy: 'transaction_date DESC',
    );
    return rows.map(PortfolioTransaction.fromMap).toList();
  }

  Future<void> deleteTransaction(int id) async {
    final db = await AppDatabase.instance();
    await db.delete('portfolio_transactions', where: 'id = ?', whereArgs: [id]);
  }

  Future<List<Position>> getPositions() async {
    final assets = await listAssets();
    final transactions = await listAllTransactions();
    return computePositions(assets, transactions);
  }

  /// Mesma lógica de `get_portfolio_positions`
  /// (`desktop/src-tauri/src/commands/transaction.rs`): quantidade líquida
  /// é compra menos venda; preço médio é a média ponderada só das compras
  /// (vendas não reduzem a base de custo — não é FIFO, é a mesma
  /// simplificação deliberada do desktop). Função pura, sem banco, pra
  /// facilitar teste.
  static List<Position> computePositions(
    List<Asset> assets,
    List<PortfolioTransaction> transactions,
  ) {
    final positions = <Position>[];

    for (final asset in assets) {
      var netQuantity = 0.0;
      var buyQuantitySum = 0.0;
      var buyValueSum = 0.0;
      final custodiaQty = <int?, double>{};

      for (final tx in transactions.where((t) => t.assetId == asset.id)) {
        switch (tx.type) {
          case TransactionType.compra:
            netQuantity += tx.quantity!;
            buyQuantitySum += tx.quantity!;
            buyValueSum += tx.totalValue;
            custodiaQty[tx.custodiaId] =
                (custodiaQty[tx.custodiaId] ?? 0) + tx.quantity!;
          case TransactionType.venda:
            netQuantity -= tx.quantity!;
            custodiaQty[tx.custodiaId] =
                (custodiaQty[tx.custodiaId] ?? 0) - tx.quantity!;
          case TransactionType.transferencia:
            custodiaQty[tx.custodiaId] =
                (custodiaQty[tx.custodiaId] ?? 0) - tx.quantity!;
            custodiaQty[tx.transferToCustodiaId] =
                (custodiaQty[tx.transferToCustodiaId] ?? 0) + tx.quantity!;
          case TransactionType.aporte:
          case TransactionType.retirada:
          case TransactionType.provento:
            // Fluxo de caixa (aporte/retirada) ou dividendo (provento) —
            // nenhum dos 3 move quantidade nem entra no preço médio, mesma
            // regra do desktop (`get_portfolio_positions`).
            break;
        }
      }

      positions.add(
        Position(
          asset: asset,
          netQuantity: netQuantity,
          averageBuyPrice: buyQuantitySum > 0
              ? buyValueSum / buyQuantitySum
              : null,
          byCustodia: custodiaQty.entries
              .map(
                (e) => CustodiaBreakdown(custodiaId: e.key, quantity: e.value),
              )
              .toList(),
        ),
      );
    }

    return positions;
  }
}
