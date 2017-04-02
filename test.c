#include <msp430x20x2.h>

const char query_str[] = "codepls";

void print_code();

int main() {
  // turn off WDT
  WDTCTL = WDTPW + WDTHOLD;

  // 1 MHz clock
  DCOCTL = 0;
  BCSCTL1 = CALBC1_1MHZ;
  DCOCTL = CALDCO_1MHZ;

  // Enable pullup on P1.3
  P1OUT &= 0x00;
  P1DIR &= 0x00;
  P1REN |= BIT3;
  P1OUT |= BIT3;

  // Configure P1.1 and P1.2 as UART lines
  P1SEL |= RXD | TXD ; // P1.1 = RXD, P1.2=TXD
  P1SEL2 |= RXD | TXD ; // P1.1 = RXD, P1.2=TXD

  // Enable pin interrupt on P1.3
  P1IE |= BIT3;
  P1IES |= BIT3;
  P1IFG &= ~BIT3;

  // Configure the UART
  UCA0CTL1 |= UCSSEL_2; // SMCLK
  UCA0BR0 = 0x08; // 1MHz 115200
  UCA0BR1 = 0x00; // 1MHz 115200
  UCA0MCTL = UCBRS2 + UCBRS0; // Modulation UCBRSx = 5
  UCA0CTL1 &= ~UCSWRST; // **Initialize USCI state machine**
  /// UC0IE |= UCA0RXIE; // Enable USCI_A0 RX interrupt

  _BIS_GR(GIE);

  int str_pos = 0;
  while (1) {
    while (!(IFG2 & UCA0RXIFG));
    if (UCA0RXBUF == query_str[str_pos]) {
      ++str_pos;
    } else {
      str_pos = 0;
    }
    if (str_pos > 7) {
      print_code();
    }
  }
}

#pragma vector=TIMERA0_VECTOR
__interrupt void timera_isr() {
}

#pragma vector=PORT01_VECTOR
__interrupt void port1_isr() {
}

const char s[] = {};
const int slen = 0;

struct deflate_state {
  int pos;
};

struct deflate_ret {
  enum {
    LIT,
    LEN
  } type;
  int val;
};

void print_code() {
}

///AUTOGEN START
///AUTOGEN END
