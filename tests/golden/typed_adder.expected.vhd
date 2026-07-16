library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_typed_adder is
  port (
    gl_p0_a : in unsigned(7 downto 0);
    gl_p1_y : out unsigned(7 downto 0)
  );
end entity gl_m0_typed_adder;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_typed_adder is
  signal gl_s1_y : unsigned(7 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_comb_0 : process(all)
  begin
    gl_s1_y <= (gl_p0_a + resize(unsigned'(x"0000000000000001"), 8));
  end process gl_comb_0;
  gl_p1_y <= gl_s1_y;
end architecture rtl;
