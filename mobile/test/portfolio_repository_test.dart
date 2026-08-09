import 'package:flutter_test/flutter_test.dart';

import 'package:anchor_mobile/models/asset.dart';
import 'package:anchor_mobile/models/portfolio_transaction.dart';
import 'package:anchor_mobile/services/portfolio_repository.dart';

void main() {
  final asset = Asset(
    id: 1,
    ticker: 'PETR4',
    name: 'Petrobras',
    assetClass: AssetClass.acaoBr,
    currency: 'BRL',
    createdAt: DateTime(2026, 1, 1),
  );

  PortfolioTransaction tx({
    required TransactionType type,
    required double quantity,
    required double unitPrice,
  }) {
    return PortfolioTransaction(
      assetId: 1,
      type: type,
      quantity: quantity,
      unitPrice: unitPrice,
      totalValue: quantity * unitPrice,
      date: DateTime(2026, 1, 1),
      createdAt: DateTime(2026, 1, 1),
    );
  }

  test('ativo sem transação nenhuma não aparece com posição', () {
    final positions = PortfolioRepository.computePositions([asset], []);
    expect(positions, hasLength(1));
    expect(positions.first.netQuantity, 0);
    expect(positions.first.averageBuyPrice, isNull);
  });

  test('compra única define quantidade e preço médio', () {
    final positions = PortfolioRepository.computePositions(
      [asset],
      [tx(type: TransactionType.compra, quantity: 100, unitPrice: 30)],
    );

    expect(positions.first.netQuantity, 100);
    expect(positions.first.averageBuyPrice, 30);
  });

  test('venda parcial reduz quantidade líquida mas não mexe no preço médio', () {
    final positions = PortfolioRepository.computePositions(
      [asset],
      [
        tx(type: TransactionType.compra, quantity: 100, unitPrice: 30),
        tx(type: TransactionType.venda, quantity: 40, unitPrice: 35),
      ],
    );

    expect(positions.first.netQuantity, 60);
    // Preço médio continua 30 — venda não reduz a base de custo, mesma
    // simplificação deliberada do desktop (não é FIFO/custo médio real).
    expect(positions.first.averageBuyPrice, 30);
  });

  test('duas compras em preços diferentes geram média ponderada', () {
    final positions = PortfolioRepository.computePositions(
      [asset],
      [
        tx(type: TransactionType.compra, quantity: 100, unitPrice: 20),
        tx(type: TransactionType.compra, quantity: 100, unitPrice: 40),
      ],
    );

    expect(positions.first.netQuantity, 200);
    expect(positions.first.averageBuyPrice, 30);
  });

  test('aporte/retirada não aparecem em nenhuma posição', () {
    final cashFlow = [
      PortfolioTransaction(
        type: TransactionType.aporte,
        totalValue: 1000,
        date: DateTime(2026, 1, 1),
        createdAt: DateTime(2026, 1, 1),
      ),
      PortfolioTransaction(
        type: TransactionType.retirada,
        totalValue: 200,
        date: DateTime(2026, 1, 2),
        createdAt: DateTime(2026, 1, 2),
      ),
    ];

    final positions = PortfolioRepository.computePositions([asset], cashFlow);

    expect(positions.first.netQuantity, 0);
    expect(positions.first.averageBuyPrice, isNull);
  });

  test('provento não altera quantidade nem preço médio do ativo', () {
    final positions = PortfolioRepository.computePositions(
      [asset],
      [
        tx(type: TransactionType.compra, quantity: 100, unitPrice: 30),
        PortfolioTransaction(
          assetId: 1,
          type: TransactionType.provento,
          totalValue: 50,
          date: DateTime(2026, 2, 1),
          createdAt: DateTime(2026, 2, 1),
        ),
      ],
    );

    expect(positions.first.netQuantity, 100);
    expect(positions.first.averageBuyPrice, 30);
  });
}
