import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:anchor_mobile/main.dart';

void main() {
  testWidgets('App abre na tela de busca de cotação', (WidgetTester tester) async {
    await tester.pumpWidget(const AnchorApp());

    expect(find.text('Anchor'), findsWidgets);
    expect(find.widgetWithText(TextField, 'Ticker (Ação BR)'), findsOneWidget);
    expect(find.widgetWithText(FilledButton, 'Buscar'), findsOneWidget);
  });
}
