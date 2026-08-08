import 'package:flutter/material.dart';

import 'screens/quote_search_screen.dart';
import 'theme.dart';

void main() {
  runApp(const AnchorApp());
}

class AnchorApp extends StatelessWidget {
  const AnchorApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Anchor',
      theme: buildAnchorTheme(),
      home: const QuoteSearchScreen(),
      debugShowCheckedModeBanner: false,
    );
  }
}
